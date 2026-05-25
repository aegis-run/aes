#[tokio::main]
async fn main() -> aes::Result {
    let init = aes::EnvProcessInit::new().map_err(|err| {
        eprintln!("error: failed to initialize process: {err}");
        aes::RunResult::IoError
    })?;

    Ok(aes::run(init).await)
}
