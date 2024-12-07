use std::process::Command;

pub fn main() {
    Command::new("npx")
        .args(&[
            "@tailwindcss/cli",
            "-i",
            ".input.css",
            "-o",
            "./assets/tailwind.css",
        ])
        .status()
        .unwrap();
}
