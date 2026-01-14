pub struct Bcolors;

impl Bcolors {
    pub fn new() -> Self {
        Bcolors
    }
    
    pub fn header(&self, text: &str) -> String {
        format!("\x1b[95m{}\x1b[0m", text)
    }
    
    pub fn okblue(&self, text: &str) -> String {
        format!("\x1b[94m{}\x1b[0m", text)
    }
    
    pub fn okgreen(&self, text: &str) -> String {
        format!("\x1b[92m{}\x1b[0m", text)
    }
    
    pub fn warning(&self) -> &'static str {
        "\x1b[93m"
    }
    
    pub fn fail(&self, text: &str) -> String {
        format!("\x1b[91m{}\x1b[0m", text)
    }
    
    pub fn endc(&self) -> &'static str {
        "\x1b[0m"
    }
    
    pub fn bold(&self) -> &'static str {
        "\x1b[1m"
    }
    
    pub fn inverse(&self, text: &str) -> String {
        format!("\x1b[7m{}\x1b[0m", text)
    }
    
    pub fn underline(&self, text: &str) -> String {
        format!("\x1b[4m{}\x1b[0m", text)
    }
}

