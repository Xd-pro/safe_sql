use std::{fs, collections::HashMap, error::Error, env};
mod sqlfile;
mod php;
mod php_lib;

fn main() -> Result<(), &'static dyn Error> {
    let mut out = fs::read_to_string("base.php").unwrap() + "class Transaction extends TransactionBase {";

    let mut base: HashMap<String, String> = HashMap::new();

    if let Ok(entries) = fs::read_dir("queries") {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(ftype) = entry.file_type() {
                    if ftype.is_file() && entry.file_name().to_str().unwrap().to_owned().ends_with(".sql") {
                        let text = fs::read_to_string("queries/".to_string() + entry.file_name().to_str().unwrap()).unwrap();
                        base = sqlfile::lex_2(entry.file_name().to_str().to_owned().unwrap().replace(".sql", ""), sqlfile::lex(text).unwrap(), base).unwrap();
                    }
                }
            }
        }
    }


    for (name, sql) in &base {
        let tokens = sqlfile::lex_sql(sql.clone());
        out.push_str(&php::generate_method(&name, &tokens).to_string());
    }

    out.push('}');

    for (name, sql) in &base {
        let tokens = sqlfile::lex_sql(sql.clone());
        out.push_str(&php::generate_return_type(&name, &tokens));
    }

    let mut it = env::args();
    it.next();
    let out_location = match it.next() {
        Some(a) => {a}
        None => {"out.php".to_string()}
    };

    println!("{:?}", out_location);

    let _ = fs::write(out_location, out);

    Ok(())
    
}

/*
    Steps:
        read sql file
        find variables and their types
        connect to test db
        generate php classes
*/