use keyring::Entry;

fn main() {
    let entry = Entry::new("vaultseek", "api_key").unwrap();
    println!("Setting password...");
    match entry.set_password("test_key") {
        Ok(_) => println!("Success setting!"),
        Err(e) => println!("Error setting: {:?}", e),
    }
    
    match entry.get_password() {
        Ok(pw) => println!("Read password: {}", pw),
        Err(e) => println!("Error reading: {:?}", e),
    }
}
