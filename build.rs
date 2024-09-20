fn main() {
    let git_rev_parse_output =
        std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();
    let git_rev_parse_cmd_output =
        String::from_utf8(git_rev_parse_output.stdout[..6].to_vec()).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_rev_parse_cmd_output);
}
