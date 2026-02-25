#[derive(macros::FromEnv)]
struct Settings {
    #[from_env(default = "x")]
    name: Option<String>,
}

fn main() {}
