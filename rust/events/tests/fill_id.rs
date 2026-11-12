use phoenix_rise_events::MarketEvent;
use phoenix_rise_events::fill_id::{
    EventParserCtx, FillIdDerivationError, collect_parsed_pnls_with_fill_id,
    generate_fill_id_from_pnl, generate_fill_id_v3, generate_fill_id_v3_from_bytes,
};
use phoenix_rise_events::market_events::{PnLEvent, SlotContextEvent, Symbol};
use phoenix_rise_math::{SignedBaseLots, SignedQuoteLots};

#[test]
fn api_fill_id_uses_raw_bytes_signed_lots_and_phoenix_ordinal() {
    let mut signature = [0; 64];
    signature[0] = 7;
    signature[63] = 8;
    let mut trader = [0; 32];
    trader[0] = 11;
    trader[31] = 12;
    let mut signature_text = [0; five8::BASE58_ENCODED_64_MAX_LEN];
    let len = usize::from(five8::encode_64(&signature, &mut signature_text));
    let signature_text = std::str::from_utf8(&signature_text[..len]).unwrap();
    let mut trader_text = [0; five8::BASE58_ENCODED_32_MAX_LEN];
    let len = usize::from(five8::encode_32(&trader, &mut trader_text));
    let trader_text = std::str::from_utf8(&trader_text[..len]).unwrap();

    let expected = "4d11004b-3a3d-312a-acfd-cd9e2d3d5f4e";
    assert_eq!(
        generate_fill_id_v3_from_bytes(&signature, 7, &trader, 42, 100, 150, -25).to_string(),
        expected,
    );
    assert_eq!(
        generate_fill_id_v3(signature_text, 7, trader_text, 42, 100, 150, -25)
            .unwrap()
            .to_string(),
        expected,
    );
    let pnl = PnLEvent {
        trader,
        asset_id: 42,
        asset_symbol: Symbol::new("SOL").unwrap(),
        realized_pnl: SignedQuoteLots::new(-25),
        funding_payment: SignedQuoteLots::new(9),
        base_lots_before: SignedBaseLots::new(100),
        base_lots_after: SignedBaseLots::new(150),
        virtual_quote_lots_before: SignedQuoteLots::new(-2_000),
        virtual_quote_lots_after: SignedQuoteLots::new(-3_000),
    };
    let context = EventParserCtx {
        signature: signature_text.to_owned(),
        top_level_instruction_index: 3,
        stack_height: Some(4),
        phoenix_instruction_index: 7,
        instruction_type: "PlaceMarketOrder".to_owned(),
    };
    let events = [
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 1,
            slot: 2,
        }),
        MarketEvent::PnL(pnl),
    ];
    let pnls = collect_parsed_pnls_with_fill_id(&context, &events).unwrap();
    assert_eq!(pnls.len(), 1);
    assert_eq!(pnls[0].fill_id.to_string(), expected);
    assert_eq!(pnls[0].ctx.phoenix_instruction_index, 7);
    assert_ne!(
        generate_fill_id_from_pnl(signature_text, 3, &pnl).unwrap(),
        pnls[0].fill_id
    );

    for invalid in ["not base58!", "111"] {
        assert!(matches!(
            generate_fill_id_from_pnl(invalid, 7, &pnl),
            Err(FillIdDerivationError::InvalidSignatureBase58(_))
        ));
        assert!(matches!(
            generate_fill_id_v3(signature_text, 7, invalid, 42, 100, 150, -25),
            Err(FillIdDerivationError::InvalidTraderPdaBase58(_))
        ));
    }
}
