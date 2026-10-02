#[tokio::main(flavor = "current_thread")]
async fn main() {
    if recollect_agent::plugin_runtime::run(&std::env::args().skip(1).collect::<Vec<_>>())
        .await
        .is_err()
    {
        eprintln!(
            "Recollect plugin operation failed. Check connection, access and plugin configuration."
        );
        std::process::exit(1);
    }
}
