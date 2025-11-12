use defaults_rs::{Domain, PrefValue, Preferences};

fn main() {
    // Reading a value
    let domain = Domain::User("com.apple.dock".to_string());

    let tilesize = if let Ok(tilesize) = Preferences::read(domain, "tilesize") {
        match tilesize {
            PrefValue::Integer(value) => {
                println!("Dock tile size {value}");
                Some(value)
            }
            _ => None,
        }
    } else {
        None
    };

    // Writing a value
    println!("Making a BIG DOCK!");

    let domain = Domain::User("com.apple.dock".to_string());
    if let Err(_) = Preferences::write(domain.clone(), "tilesize", PrefValue::Integer(100)) {
        println!("Error writing tilesize");
    }

    // Restoring value
    if tilesize.is_some() {
        println!("Restoring dock tile size");
        if let Err(_) =
            Preferences::write(domain, "tilesize", PrefValue::Integer(tilesize.unwrap()))
        {
            println!("Error restoring tilesize");
        }
    }
}
