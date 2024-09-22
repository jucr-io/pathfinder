fn main() {
    let git_rev_parse_output =
        std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();
    let git_rev_parse_cmd_output =
        String::from_utf8(git_rev_parse_output.stdout.to_vec()).unwrap_or_default();

    println!("cargo:rustc-env=GIT_HASH={}", git_rev_parse_cmd_output);
}
