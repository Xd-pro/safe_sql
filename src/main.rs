use std::{fs, collections::HashMap, error::Error, env};

use sqlfile::{SqlToken, lex_sql};
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

    let mut tokens: HashMap<String, Vec<SqlToken>> = HashMap::new();

    for (name, sql) in &base {
        tokens.insert(name.to_string(), lex_sql(sql.to_string()));
        if sql == "" {
            panic!("Syntax error in {}", name);
        }
        out.push_str(&php::generate_method(&name, &tokens[name]).to_string());
    }

    out.push('}');

    for (name, _sql) in &base {
        out.push_str(&php::generate_return_type(&name, &tokens[name]));
    }

    for (name, _sql) in &base {
        out.push_str(&php::generate_async_transaction(&name, &tokens[name]).to_string());
    }

    let mut it = env::args();
    it.next();
    let out_location = match it.next() {
        Some(a) => {a}
        None => {"out.php".to_string()}
    };

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