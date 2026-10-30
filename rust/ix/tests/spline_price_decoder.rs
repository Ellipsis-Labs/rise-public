use phoenix_rise_ix::discriminants::PhoenixInstruction;
use phoenix_rise_ix::prelude::{
    SplineUpdateAccounts, UpdateSplinePriceWithMetadataParams,
    create_update_spline_price_with_metadata_ix,
};
use solana_pubkey::Pubkey;

#[test]
fn spline_instruction_encoding_is_exact_and_decodes_every_input_field() {
    let client_order_id = std::array::from_fn(|index| index as u8);
    for slot in [None, Some(0), Some(u64::MAX)] {
        for (bid_width, ask_width) in [
            (None, None),
            (Some(0), None),
            (None, Some(u32::MAX)),
            (Some(17), Some(29)),
        ] {
            let params = UpdateSplinePriceWithMetadataParams {
                new_mid_price: 123_456,
                refresh_regions: true,
                user_sequence_number: 42,
                override_sequence_number: false,
                client_order_id,
                user_update_slot: slot,
                bid_width,
                ask_width,
            };
            let accounts = SplineUpdateAccounts::builder()
                .signer(Pubkey::new_from_array([1; 32]))
                .trader_account(Pubkey::new_from_array([2; 32]))
                .spline_collection(Pubkey::new_from_array([3; 32]))
                .build()
                .unwrap();
            let ix = create_update_spline_price_with_metadata_ix(accounts, params.clone()).unwrap();
            // Independently specify the wire bytes, including the
            // discriminator.
            let mut expected = vec![153, 162, 192, 201, 188, 88, 30, 224];
            expected.extend_from_slice(&123_456_u32.to_le_bytes());
            let flags = 1u32
                | (if slot.is_some() { 4 } else { 0 })
                | (if bid_width.is_some() { 8 } else { 0 })
                | (if ask_width.is_some() { 16 } else { 0 });
            expected.extend_from_slice(&flags.to_le_bytes());
            expected.extend_from_slice(&42_u64.to_le_bytes());
            expected.extend_from_slice(&slot.unwrap_or(0).to_le_bytes());
            expected.extend_from_slice(&bid_width.unwrap_or(0).to_le_bytes());
            expected.extend_from_slice(&ask_width.unwrap_or(0).to_le_bytes());
            expected.extend_from_slice(&client_order_id);
            assert_eq!(ix.data, expected);
            assert_eq!(params.encode(), expected[8..]);
            let mut instruction_data = ix.data;
            let decoded =
                UpdateSplinePriceWithMetadataParams::decode_instruction_data(&instruction_data)
                    .unwrap();
            assert_eq!(decoded.client_order_id, client_order_id);
            assert_eq!(decoded, params);
            assert_eq!(
                UpdateSplinePriceWithMetadataParams::decode(&instruction_data[8..]).unwrap(),
                params
            );

            for tail_length in 0..16 {
                let decoded = UpdateSplinePriceWithMetadataParams::decode_instruction_data(
                    &instruction_data[..8 + 32 + tail_length],
                )
                .unwrap();
                assert_eq!(decoded.client_order_id, [0; 16]);
                assert_eq!(decoded.new_mid_price, params.new_mid_price);
                assert_eq!(decoded.bid_width, params.bid_width);
                assert_eq!(decoded.ask_width, params.ask_width);
            }

            instruction_data[0] ^= 1;
            assert!(
                UpdateSplinePriceWithMetadataParams::decode_instruction_data(&instruction_data,)
                    .is_err()
            );
        }
    }
}

#[test]
fn public_spline_decoder_rejects_missing_discriminator_and_truncated_payload() {
    for length in 0..8 {
        assert!(
            UpdateSplinePriceWithMetadataParams::decode_instruction_data(&vec![0; length],)
                .is_err()
        );
    }
    for payload_length in 0..32 {
        let mut instruction_data = PhoenixInstruction::UpdateSplinePriceWithMetadata
            .discriminant()
            .to_vec();
        instruction_data.extend_from_slice(&vec![0; payload_length]);
        assert!(
            UpdateSplinePriceWithMetadataParams::decode_instruction_data(&instruction_data,)
                .is_err()
        );
    }
}

#[test]
fn metadata_builder_refreshes_regions_by_default_like_update_spline_price() {
    let params = UpdateSplinePriceWithMetadataParams::builder()
        .new_mid_price(1000)
        .user_sequence_number(1)
        .build()
        .unwrap();
    assert!(params.refresh_regions);
    let params = UpdateSplinePriceWithMetadataParams::builder()
        .new_mid_price(1000)
        .user_sequence_number(1)
        .refresh_regions(false)
        .build()
        .unwrap();
    assert!(!params.refresh_regions);
}
