#[test]
fn http_builder_exposes_trace_context_provider_when_enabled() {
    let _builder = phoenix_rise::api::PhoenixHttpClient::builder("https://api.phoenix.trade")
        .with_trace_context_provider(|| unreachable!("compile-only trace context provider"));
}
