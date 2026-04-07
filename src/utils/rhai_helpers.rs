use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, RenderErrorReason,
};
use rhai::serde::{from_dynamic, to_dynamic};
use rhai::{Dynamic, Engine, Scope};
use serde_json::{Value, json};
use std::sync::Arc;

const HELPER_REGISTRY_FN: &str = "handlebars_helpers";

pub fn register_handlebars_helpers(
    handlebars: &mut Handlebars<'_>,
    script: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let helper_names: Vec<String> = discover_handlebars_helpers(script)?;
    let shared_script: Arc<str> = Arc::from(script.to_owned());

    for name in helper_names {
        let registration_name = name.clone();
        let helper = ScriptHelper {
            script: Arc::clone(&shared_script),
            function_name: name,
        };
        handlebars.register_helper(&registration_name, Box::new(helper));
    }

    Ok(())
}

struct ScriptHelper {
    script: Arc<str>,
    function_name: String,
}

impl HelperDef for ScriptHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        context: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let engine = Engine::new();
        let ast = engine
            .compile(&self.script)
            .map_err(|err| render_error(err.to_string()))?;
        let payload = build_payload(helper, context.data());
        let input = to_dynamic(&payload).map_err(|err| render_error(err.to_string()))?;
        let result = engine
            .call_fn::<Dynamic>(&mut Scope::new(), &ast, &self.function_name, (input,))
            .map_err(|err| render_error(err.to_string()))?;

        let rendered = match from_dynamic::<Value>(&result) {
            Ok(value) => render_value(&value),
            Err(_) => result.to_string(),
        };

        out.write(&rendered)?;
        Ok(())
    }
}

fn discover_handlebars_helpers(script: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let engine = Engine::new();
    let ast = engine.compile(script)?;

    match engine.call_fn::<Dynamic>(&mut Scope::new(), &ast, HELPER_REGISTRY_FN, ()) {
        Ok(result) => from_dynamic(&result).map_err(Into::into),
        Err(err) if is_missing_function(&err.to_string(), HELPER_REGISTRY_FN) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

fn build_payload(helper: &Helper<'_>, root: &Value) -> Value {
    let args = helper
        .params()
        .iter()
        .map(|value| value.value().clone())
        .collect::<Vec<_>>();
    let hash = helper
        .hash()
        .iter()
        .map(|(key, value)| (key.to_string(), value.value().clone()))
        .collect::<serde_json::Map<String, Value>>();

    json!({
        "args": args,
        "hash": hash,
        "root": root,
    })
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

fn render_error(message: impl Into<String>) -> RenderError {
    RenderErrorReason::Other(message.into()).into()
}

fn is_missing_function(message: &str, function_name: &str) -> bool {
    message.contains("Function not found") && message.contains(function_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_rhai_helpers_from_script() {
        let script = r#"
            fn shout(helper) {
                helper["args"][0] + "!"
            }

            fn handlebars_helpers() {
                ["shout"]
            }
        "#;

        let mut handlebars = Handlebars::new();
        register_handlebars_helpers(&mut handlebars, script).unwrap();

        let rendered = handlebars
            .render_template("{{shout name}}", &serde_json::json!({ "name": "db" }))
            .unwrap();

        assert_eq!(rendered, "db!");
    }
}