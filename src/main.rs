use std::{collections::HashMap, env, fs, io, path::Path, process::exit};

use sqlfile::{SqlToken, lex_sql};
mod sqlfile;
mod php;
mod php_lib;

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    out: String,
    queries_dir: String,
    namespace: String
}

fn get_config() -> Result<Config, String> {
    let mut args = env::args();
    let _ = args.next();
    let arg = args.next();
    let path;
    let a: String;
    if let Some(arg) = arg {
        a = arg;
        path = Path::new(&a);
    } else {
        path = Path::new("safe_sql.toml");
    }
    if !path.exists() {
        return Err(format!("Config file {} not found", path.as_os_str().to_str().unwrap()))
    }
    
    println!("{:?}", path);

    Ok(toml::from_str(&fs::read_to_string(path).unwrap()).unwrap())
    
}

fn main() -> io::Result<()> {

    let config = match get_config() {
        Err(str) => {
            println!("{}", str);
            exit(1);
        }
        Ok(t) => t
    };

    let ns = "namespace ".to_owned() + &config.namespace + ";";

    let mut out = include_str!("../base.php").replace("//%%NAMESPACE%%", &ns) + "class Transaction extends TransactionBase {";

    let mut base: HashMap<String, String> = HashMap::new();

    if let Ok(entries) = fs::read_dir(&config.queries_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(ftype) = entry.file_type() {
                    if ftype.is_file() && entry.file_name().to_str().unwrap().to_owned().ends_with(".sql") {
                        let text = fs::read_to_string(config.queries_dir.clone() + "/" + entry.file_name().to_str().unwrap()).unwrap();
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

    fs::write(config.out, out)
    
}

/*
    Steps:
        read sql file
        find variables and their types
        connect to test db
        generate php classes
*/