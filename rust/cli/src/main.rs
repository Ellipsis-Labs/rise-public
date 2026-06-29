use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    phoenix_rise_cli::run_from_args().await
}
