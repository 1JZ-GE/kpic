mod daemon;

fn main() {
    daemon::run_blocking().expect("daemon failed");
}