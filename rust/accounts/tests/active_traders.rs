use phoenix_rise_accounts::active_trader_buffer::{ActiveTraderBuffer, TraderPositionId};
use phoenix_rise_accounts::discriminants::PhoenixAccount;
use phoenix_rise_accounts::global_trader_index::GlobalTraderIndex;
use phoenix_rise_accounts::trader::{LOCKED_DEPOSIT_ASSET_INDEX, NATIVE_SOL_ASSET_INDEX};
use solana_pubkey::Pubkey;

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn gti_fixture() -> Vec<Vec<u8>> {
    // Header, two nodes; overflow arena 1, two slots; registered empty arena 2.
    let mut head = vec![0; 224];
    head[..8].copy_from_slice(&PhoenixAccount::GlobalTraderIndexHeader.discriminant());
    write_u64(&mut head, 8, 17);
    write_u64(&mut head, 16, 250);
    write_u32(&mut head, 48, 3);
    head[52..54].copy_from_slice(&3u16.to_le_bytes());
    head[54..56].copy_from_slice(&2u16.to_le_bytes());
    write_u32(&mut head, 56, 2);
    write_u32(&mut head, 60, 4);
    write_u32(&mut head, 64, 4);
    write_u32(&mut head, 80, 2);
    // Root #2 (key 20), left #1 (key 10), right #3 (key 30).
    write_u32(&mut head, 104, 2);
    head[112..144].fill(10);
    write_u64(&mut head, 144, 111);
    write_u32(&mut head, 160, 1);
    write_u32(&mut head, 164, 3);
    head[176..208].fill(20);
    write_u64(&mut head, 208, 222);

    let mut arena = vec![0; 160];
    arena[..8].copy_from_slice(&PhoenixAccount::GlobalTraderIndexArenaHeader.discriminant());
    arena[8..10].copy_from_slice(&1u16.to_le_bytes());
    write_u32(&mut arena, 40, 2);
    arena[48..80].fill(30);
    write_u64(&mut arena, 80, 333);
    // Unallocated capacity is not a live trader, even with a plausible key.
    arena[112..144].fill(40);

    let mut empty = vec![0; 32];
    empty[..8].copy_from_slice(&PhoenixAccount::GlobalTraderIndexArenaHeader.discriminant());
    empty[8..10].copy_from_slice(&2u16.to_le_bytes());
    vec![head, arena, empty]
}

#[test]
fn gti_resolves_overflow_node_indices_without_reading_unused_capacity() {
    let buffers = gti_fixture();
    let gti =
        GlobalTraderIndex::try_from_account_buffers(buffers.iter().map(Vec::as_slice)).unwrap();
    assert_eq!(gti.header().sequence_number().sequence_number, 17);
    assert_eq!(gti.header().sequence_number().last_update_slot, 250);
    assert_eq!(gti.len(), 3);
    assert_eq!(gti.capacity(), 4);
    let trader = Pubkey::new_from_array([30; 32]);
    assert_eq!(gti.index(&trader), Some(3));
    assert_eq!(
        gti.get(&trader).unwrap().quote_lot_collateral.to_string(),
        "333"
    );
    assert_eq!(
        gti.get_by_index(3),
        Some((trader, gti.get(&trader).unwrap()))
    );
    assert!(gti.get_by_index(0).is_none());
    assert!(gti.get_by_index(4).is_none());
    assert!(!gti.contains(&Pubkey::new_from_array([40; 32])));
    assert_eq!(
        gti.iter_with_indices()
            .map(|(i, _, _)| i)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let unaligned: Vec<_> = buffers
        .iter()
        .map(|bytes| {
            let mut buffer = vec![0];
            buffer.extend_from_slice(bytes);
            buffer
        })
        .collect();
    let gti =
        GlobalTraderIndex::try_from_account_buffers(unaligned.iter().map(|bytes| &bytes[1..]))
            .unwrap();
    assert_eq!(gti.index(&trader), Some(3));
    // A freed node below the bump boundary still carries its previous key,
    // which can also be present in another live node. Check reachability.
    let mut freed = buffers;
    write_u32(&mut freed[0], 60, 5);
    write_u32(&mut freed[1], 100, u32::MAX);
    freed[1][112..144].fill(20);
    let gti = GlobalTraderIndex::try_from_account_buffers(freed.iter().map(Vec::as_slice)).unwrap();
    assert!(gti.get_by_index(4).is_none());
    assert_eq!(
        gti.get_by_index(2).unwrap().0,
        Pubkey::new_from_array([20; 32])
    );
}

#[test]
fn gti_rejects_incomplete_arenas_and_corrupt_tree_links() {
    let original = gti_fixture();
    assert!(
        GlobalTraderIndex::try_from_account_buffers(original[..2].iter().map(Vec::as_slice))
            .is_err()
    );
    assert!(GlobalTraderIndex::try_from_account_buffers(std::iter::empty()).is_err());
    for (buffer, offset, value) in [
        (0, 0, 0),     // Wrong account discriminator.
        (1, 8, 2),     // Wrong overflow arena index.
        (0, 48, 2),    // Reachable nodes exceed declared count.
        (0, 56, 0),    // Zero page size would divide by zero.
        (0, 60, 99),   // Bump pointer exceeds allocated capacity.
        (0, 160, 2),   // Root links to itself.
        (0, 164, 99),  // Child pointer outside arena bounds.
        (1, 40, 1),    // Overflow child has the wrong parent.
        (0, 112, 255), // Left key sorts after the root.
    ] {
        let mut buffers = original.clone();
        write_u32(&mut buffers[buffer], offset, value);
        assert!(
            GlobalTraderIndex::try_from_account_buffers(buffers.iter().map(Vec::as_slice)).is_err(),
            "buffer {buffer}, offset {offset}"
        );
    }
    let mut buffers = original.clone();
    buffers[1].pop();
    assert!(
        GlobalTraderIndex::try_from_account_buffers(buffers.iter().map(Vec::as_slice)).is_err()
    );
    let mut buffers = original;
    buffers.swap(1, 2);
    assert!(
        GlobalTraderIndex::try_from_account_buffers(buffers.iter().map(Vec::as_slice)).is_err()
    );
}

fn atb_fixture() -> Vec<u8> {
    let mut bytes = vec![0; 96 + 8 * 88];
    bytes[..8].copy_from_slice(&PhoenixAccount::ActiveTraderBufferHeader.discriminant());
    write_u32(&mut bytes, 48, 8);
    bytes[52..54].copy_from_slice(&1u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&1u16.to_le_bytes());
    write_u32(&mut bytes, 56, 8);
    write_u32(&mut bytes, 60, 9);
    write_u32(&mut bytes, 64, 9);
    write_u32(&mut bytes, 80, 4);
    let nodes = [
        // left, right, parent, trader_id, asset_id
        (0, 0, 2, 1, 7),
        (1, 3, 4, 1, 8),
        (0, 0, 2, 1, 0xFF00_0001),
        (2, 6, 0, 1, 0xFFFE_0002),
        (0, 0, 6, 1, NATIVE_SOL_ASSET_INDEX),
        (5, 7, 4, 1, LOCKED_DEPOSIT_ASSET_INDEX),
        (0, 8, 6, 1, 0xFFFF_0002),
        (0, 0, 7, 2, 7),
    ];
    for (i, (left, right, parent, trader, asset)) in nodes.into_iter().enumerate() {
        let node = 96 + i * 88;
        write_u32(&mut bytes, node, left);
        write_u32(&mut bytes, node + 4, right);
        write_u32(&mut bytes, node + 8, parent);
        write_u32(&mut bytes, node + 16, trader);
        write_u32(&mut bytes, node + 20, asset);
    }
    // Position #1: short 12 lots, quote 300, funding -4; sequence byte with
    // signed i56 accumulated funding immediately following its sequence byte.
    write_u64(&mut bytes, 120, (-12i64) as u64);
    write_u64(&mut bytes, 128, 300);
    write_u64(&mut bytes, 136, (-4i64) as u64);
    bytes[144] = 7;
    bytes[145..152].copy_from_slice(&(-9i64).to_le_bytes()[..7]);
    write_u32(&mut bytes, 152, 101);
    write_u32(&mut bytes, 156, 23);
    write_u32(&mut bytes, 160, 99);
    write_u32(&mut bytes, 164, 24);
    write_u32(&mut bytes, 168, 4);
    write_u32(&mut bytes, 172, 5);
    write_u32(&mut bytes, 176, 6);
    write_u32(&mut bytes, 180, 7);
    // SPL collateral #4, native SOL #5, locked deposit #6.
    write_u64(&mut bytes, 384, 1234);
    write_u64(&mut bytes, 392, 2000);
    write_u64(&mut bytes, 472, 1_000_000_000);
    write_u64(&mut bytes, 560, 500);
    write_u64(&mut bytes, 568, 120);
    write_u64(&mut bytes, 576, 1);
    bytes
}

#[test]
fn atb_keeps_collateral_and_reserved_entries_out_of_perp_positions() {
    let bytes = atb_fixture();
    let atb = ActiveTraderBuffer::try_from_account_buffers([bytes.as_slice()]).unwrap();
    assert_eq!(atb.len(), 8);
    assert_eq!(atb.iter_all_positions().count(), 3);
    assert_eq!(
        atb.iter_positions(1)
            .map(|(id, _)| id.asset_id())
            .collect::<Vec<_>>(),
        [7, 8]
    );
    assert_eq!(atb.iter_positions(2).count(), 1);
    let position = atb
        .get_position(&TraderPositionId::new(1.into(), 7))
        .unwrap();
    assert_eq!(position.position.base_lot_position.to_string(), "-12");
    assert_eq!(position.position.position_sequence_number, 7);
    assert_eq!(
        position
            .position
            .accumulated_funding_for_active_position
            .to_string(),
        "-9"
    );
    assert_eq!(
        (
            position.lowest_ask,
            position.ask_orders_head,
            position.highest_bid,
            position.bid_orders_head
        ),
        (101, 23, 99, 24)
    );
    assert_eq!(
        (
            position.total_non_reduce_only_ask_base_lots,
            position.total_reduce_only_ask_base_lots,
            position.total_non_reduce_only_bid_base_lots,
            position.total_reduce_only_bid_base_lots
        ),
        (4, 5, 6, 7)
    );
    for asset in [
        0xFF00_0001,
        0xFFFE_0002,
        NATIVE_SOL_ASSET_INDEX,
        LOCKED_DEPOSIT_ASSET_INDEX,
        0xFFFF_0002,
    ] {
        assert!(
            atb.get_position(&TraderPositionId::new(1.into(), asset))
                .is_none()
        );
    }
    let spot = atb.get_spot_collateral(1, 0xFFFE_0002).unwrap();
    assert_eq!(
        (spot.balance, spot.max_per_trader_balance_override),
        (1234, Some(2000))
    );
    assert_eq!(atb.iter_spot_collaterals(1).collect::<Vec<_>>(), [spot]);
    let sol = atb.get_native_sol_collateral(1).unwrap();
    assert_eq!(
        (sol.balance, sol.max_per_trader_balance_override),
        (1_000_000_000, None)
    );
    let lock = atb.get_locked_deposit(1).unwrap();
    assert_eq!(lock.remaining_quote_lots(), 380);
    assert!(lock.is_isolated);
    assert!(atb.get_locked_deposit(2).is_none());
    assert!(atb.get_spot_collateral(1, 7).is_none());
    let mut unaligned = vec![0];
    unaligned.extend_from_slice(&bytes);
    let atb = ActiveTraderBuffer::try_from_account_buffers([&unaligned[1..]]).unwrap();
    assert_eq!(
        atb.get_position(&TraderPositionId::new(1.into(), 7)),
        Some(position)
    );
    // A null GTI pointer is invalid even if the key remains correctly ordered.
    let mut invalid = bytes;
    write_u32(&mut invalid, 112, 0);
    assert!(ActiveTraderBuffer::try_from_account_buffers([invalid.as_slice()]).is_err());
}
