use indexmap::IndexMap;
use crate::sqlfile::SqlToken;
use cascade::cascade;
use crate::php_lib::{Class, Visibility, Function, Param, ClassMember};

pub fn generate_return_type(class: &String, query: &Vec<SqlToken>) -> String {
    let mut has_returns = false;
    for token in query.clone() {
        match token {
            SqlToken::Return(_, _) => {
                has_returns = true;
                break;
            }
            _ => {}
        }
    }

    if !has_returns {return "".to_string()}

    let mut params: Vec<Param> = Vec::new();

    for token in query {
        match token {
            SqlToken::Return(name, type_name) => {
                params.push(Param { name: name.to_string(), param_type: type_name.to_string(), visibility: Some(Visibility::Public()) })
            }
            _ => {}
        }
    }

    let constructor = Function {
        body: Vec::new(),
        name: "__construct".to_string(),
        params,
        comment: "".to_string(),
        visibility: Some(Visibility::Public())
    };

    let members: Vec<Box<dyn ClassMember>> = vec![Box::new(constructor)];

    Class {
        name: class.to_string(),
        extends: None,
        implements: Vec::new(),
        is_abstract: false,
        members,
        comment: "".to_string()
    }.to_string()
}

pub fn generate_method<'a>(name: &String, query: &Vec<SqlToken>) -> Function {
    let mut has_returns = false;
    for token in query.clone() {
        match token {
            SqlToken::Return(_, _) => {
                has_returns = true;
                break;
            }
            _ => {}
        }
    }
    let comment = if has_returns {
        "/** @return ".to_string() + &name + "[]|\\Generator */"
    } else {
        "/** @return int */".to_string()
    };
    
    let mut vars: Vec<String> = Vec::new();
    let mut params: Vec<Param> = Vec::new();
    'outer: for token in query.clone() {
        match token {
            SqlToken::Variable(name, type_name) => {
                for param in &params {
                    if param.name == name {
                        vars.push(name);
                        continue 'outer;
                    }
                }
                params.push(Param { name: name.clone(), param_type: type_name, visibility: None });
                vars.push(name);
            }
            _ => {}
        }
    }
    let mut q_marked = "".to_string();
    let mut insert = false;
    for (i, token) in query.clone().into_iter().enumerate() {
        q_marked.push_str(match &token {
            SqlToken::Return(a, _) => a,
            SqlToken::Variable(_, _) => "?",
            SqlToken::Sql(a) => {
                if a.to_lowercase().starts_with("insert") && i == 0 {
                    insert = true;
                }
                a
            },
        })
    }
    let mut body: String = String::new();
    if has_returns {
        body.push_str("$statement = $this->db->prepare(\"");

        body.push_str(&escape(q_marked));
        body.push_str(&("\"); $statement->execute(["));
        for var in vars {
            body.push_str(&("$".to_owned() + &var + ","));
        }
        body.push_str(
            &("]); while ($res = $statement->fetch(\\PDO::FETCH_NUM)) { yield new ".to_owned()
                + &name
                + "(...$res);}"),
        );
    } else {
        body.push_str(&("$statement = $this->db->prepare(\"".to_string() + &escape(q_marked) + "\");$statement->execute(["));
        for var in vars {
            body.push_str(&("$".to_owned() + &var + ","));
        }
        body.push_str("]);");
        if insert {
            body.push_str("return $this->db->lastInsertId();")
        } else {
            body.push_str("return $statement->rowCount();")
        }
    }

    return Function {
        body: vec![Box::new(body)],
        name: name.clone(),
        params,
        comment,
        visibility: Some(Visibility::Public())
    };
}

// returns for double quotes
pub fn escape(string: String) -> String {
    let mut out = "".to_string();
    for char in string.chars() {
        match char {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\x1B' => out.push_str("\\e"),
            '\x0B' => out.push_str("\\v"),
            '\x0C' => out.push_str("\\f"),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '"' => out.push_str("\\\""),
            _ => {
                out.push(char);
            }
        }
    }
    out
}

pub fn generate_async_transaction(name: &String, query: &Vec<SqlToken>) -> Class {
    let mut body = "$out = $t->".to_string();

    let mut params: IndexMap<String, Param> = IndexMap::new();

    for token in query.clone() {
        match token {
            SqlToken::Variable(name, type_name) => {
                params.insert(name.clone(), Param { name: name.to_string(), param_type: type_name.to_string(), visibility: Some(Visibility::Private()) });
            }
            _ => {}
        }
    }

    body.push_str(name);
    body.push_str("(");
    for (_, param) in &params {
        body.push_str("$this->");
        body.push_str(&param.name);
        body.push_str(",");
    }

    let mut has_returns = false;
    for token in query {
        match token {
            SqlToken::Return(_, _) => {
                has_returns = true;
                break;
            }
            _ => {}
        }
    }

    if has_returns {
        body.push_str(");
            $rv = [];
            foreach ($out as $out) {
                $rv[]=$out;
            }
            return $rv;"
        );
    } else {
        body.push_str(");return $out;")
    }

    

    Class {
        comment: "".to_string(),
        name: cascade! { "AT_".to_string();..push_str(name); },
        extends: Some("AsyncTransaction".to_string()),
        implements: Vec::new(),
        is_abstract: false,
        members: vec![
            Box::new(Function {
                comment: "".to_string(),
                params: params.values().cloned().collect(),
                name: "__construct".to_string(),
                body: vec![],
                visibility: Some(Visibility::Public())
            }),
            Box::new(Function {
                comment: "".to_string(),
                name: "run".to_string(),
                params: vec![Param {name: "t".to_string(), param_type: "Transaction".to_string(), visibility: None}],
                body: vec![Box::new(body)],
                visibility: Some(Visibility::Public())
            })
        ]
    }
}