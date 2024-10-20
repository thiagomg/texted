pub fn get_name() -> String {
    let name = whoami::realname();
    if name.is_empty() {
        return whoami::username();
    }
    name
}