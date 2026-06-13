use keyring::Entry;
fn main() {
    let entry = Entry::new("vaultseek_test", "api_key").unwrap();
    entry.set_password("test_key").unwrap();
    let pwd = entry.get_password().unwrap();
    println!("Retrieved: {}", pwd);
}
