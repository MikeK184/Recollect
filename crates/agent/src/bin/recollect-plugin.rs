#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Most plugin commands answer with JSON on stdout and `mcp` owns the
    // stdio protocol; presentation appears only in a human terminal.
    recollect_agent::presentation::banner_quiet();
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
