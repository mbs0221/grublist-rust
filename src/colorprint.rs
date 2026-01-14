// Compatibility layer for grub_config module
pub struct Bcolors;

impl Bcolors {
    pub fn new() -> Self {
        Bcolors
    }

    pub fn okgreen(&self, text: &str) -> String {
        format!("\x1b[92m{}\x1b[0m", text)
    }

    pub fn okblue(&self, text: &str) -> String {
        format!("\x1b[94m{}\x1b[0m", text)
    }

    pub fn fail(&self, text: &str) -> String {
        format!("\x1b[91m{}\x1b[0m", text)
    }

    pub fn warning(&self) -> String {
        "\x1b[93m".to_string()
    }

    pub fn bold(&self) -> String {
        "\x1b[1m".to_string()
    }

    pub fn endc(&self) -> String {
        "\x1b[0m".to_string()
    }
}
