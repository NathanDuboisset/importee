pub fn get_relative_path(module_path: Vec<String>) -> String {
    let mut relative_path = String::new();
    for module in module_path {
        relative_path.push_str(&module);
        relative_path.push_str("/");
    }
    relative_path
}
