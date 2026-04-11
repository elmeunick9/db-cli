use crate::utils::handlebars_helpers;
use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, RenderErrorReason,
};
use rhai::serde::{from_dynamic, to_dynamic};
use rhai::{
    Array, Dynamic, Engine, EvalAltResult, ImmutableString, Module, Position, Scope,
    Shared,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

const HELPER_REGISTRY_FN: &str = "handlebars_helpers";

type DynError = Box<dyn std::error::Error>;
type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

pub fn register_handlebars_helpers(
    handlebars: &mut Handlebars<'static>,
    script: &str,
) -> Result<(), DynError> {
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

pub fn run_main_script(
    script: &str,
    context: &Value,
    input_dir: &Path,
    output_dir: &Path,
    handlebars: Arc<Handlebars<'static>>,
) -> Result<(), DynError> {
    let runtime = Arc::new(ScriptRuntime {
        handlebars,
        input_dir: input_dir.to_path_buf(),
        output_dir: output_dir.to_path_buf(),
    });
    let engine = create_engine(Some(runtime));
    let ast = engine.compile(script)?;
    let mut scope = Scope::new();
    scope.push_dynamic("context", to_dynamic(context)?);

    engine
        .call_fn::<Dynamic>(&mut scope, &ast, "main", ())
        .map(|_| ())
        .map_err(Into::into)
}

struct ScriptRuntime {
    handlebars: Arc<Handlebars<'static>>,
    input_dir: PathBuf,
    output_dir: PathBuf,
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
        let engine = create_engine(None);
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

fn discover_handlebars_helpers(script: &str) -> Result<Vec<String>, DynError> {
    let engine = create_engine(None);
    let ast = engine.compile(script)?;

    match engine.call_fn::<Dynamic>(&mut Scope::new(), &ast, HELPER_REGISTRY_FN, ()) {
        Ok(result) => from_dynamic(&result).map_err(Into::into),
        Err(err) if is_missing_function(&err.to_string(), HELPER_REGISTRY_FN) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

fn create_engine(runtime: Option<Arc<ScriptRuntime>>) -> Engine {
    let mut engine = Engine::new();
    register_hbs_module(&mut engine);

    if let Some(runtime) = runtime {
        register_runtime_functions(&mut engine, runtime);
    }

    engine
}

fn register_runtime_functions(engine: &mut Engine, runtime: Arc<ScriptRuntime>) {
    let transform_runtime = Arc::clone(&runtime);
    engine.register_fn(
        "transform",
        move |context: Dynamic, template: ImmutableString| -> RhaiResult<ImmutableString> {
            let context_value = dynamic_to_json(context)?;
            let rendered = transform_runtime
                .handlebars
                .render_template(template.as_str(), &context_value)
                .map_err(|err| rhai_error(err.to_string()))?;
            Ok(rendered.into())
        },
    );

    let load_runtime = Arc::clone(&runtime);
    engine.register_fn(
        "load",
        move |path: ImmutableString| -> RhaiResult<ImmutableString> {
            let resolved = resolve_scoped_path(&load_runtime.input_dir, path.as_str(), "template")?;
            let content = fs::read_to_string(&resolved).map_err(|err| rhai_error(err.to_string()))?;
            Ok(content.into())
        },
    );

    engine.register_fn(
        "save",
        move |path: ImmutableString, content: ImmutableString| -> RhaiResult<ImmutableString> {
            let resolved = resolve_scoped_path(&runtime.output_dir, path.as_str(), "output")?;
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent).map_err(|err| rhai_error(err.to_string()))?;
            }
            fs::write(&resolved, content.as_str()).map_err(|err| rhai_error(err.to_string()))?;
            Ok(content)
        },
    );
}

fn register_hbs_module(engine: &mut Engine) {
    let mut module = Module::new();

    module.set_native_fn("all", |values: Array| call_helper("all", values));
    module.set_native_fn("any", |values: Array| call_helper("any", values));
    module.set_native_fn("camel_case", |value: Dynamic| call_helper("camel_case", vec![value]));
    module.set_native_fn("default", |value: Dynamic, fallback: Dynamic| {
        call_helper("default", vec![value, fallback])
    });
    module.set_native_fn("eq", |values: Array| call_helper("eq", values));
    module.set_native_fn("gt", |values: Array| call_helper("gt", values));
    module.set_native_fn("gte", |values: Array| call_helper("gte", values));
    module.set_native_fn("isdefined", |value: Dynamic| call_helper("isdefined", vec![value]));
    module.set_native_fn("join", |values: Dynamic| call_helper("join", vec![values]));
    module.set_native_fn("join", |values: Dynamic, separator: Dynamic| {
        call_helper("join", vec![values, separator])
    });
    module.set_native_fn("json", || call_helper("json", Vec::new()));
    module.set_native_fn("json", |value: Dynamic| call_helper("json", vec![value]));
    module.set_native_fn("kebab_case", |value: Dynamic| call_helper("kebab_case", vec![value]));
    module.set_native_fn("lower", |value: Dynamic| call_helper("lower", vec![value]));
    module.set_native_fn("lt", |values: Array| call_helper("lt", values));
    module.set_native_fn("lte", |values: Array| call_helper("lte", values));
    module.set_native_fn("neq", |values: Array| call_helper("neq", values));
    module.set_native_fn("not", |value: Dynamic| call_helper("not", vec![value]));
    module.set_native_fn("pascal_case", |value: Dynamic| call_helper("pascal_case", vec![value]));
    module.set_native_fn("replace", |value: Dynamic, from: Dynamic, to: Dynamic| {
        call_helper("replace", vec![value, from, to])
    });
    module.set_native_fn("screaming_snake_case", |value: Dynamic| {
        call_helper("screaming_snake_case", vec![value])
    });
    module.set_native_fn("snake_case", |value: Dynamic| call_helper("snake_case", vec![value]));
    module.set_native_fn("sub", |values: Array| call_helper("sub", values));
    module.set_native_fn("sum", |values: Array| call_helper("sum", values));
    module.set_native_fn("title_case", |value: Dynamic| call_helper("title_case", vec![value]));
    module.set_native_fn("upper", |value: Dynamic| call_helper("upper", vec![value]));

    let shared: Shared<Module> = module.into();
    engine.register_static_module("hbs", shared);
}

fn call_helper(helper_name: &str, args: Vec<Dynamic>) -> RhaiResult<Dynamic> {
    let values = args
        .into_iter()
        .map(dynamic_to_json)
        .collect::<Result<Vec<_>, _>>()?;
    let result = handlebars_helpers::evaluate_common_helper(helper_name, &values, None)
        .map_err(rhai_error)?;
    to_dynamic(&result).map_err(|err| rhai_error(err.to_string()))
}

fn dynamic_to_json(value: Dynamic) -> RhaiResult<Value> {
    from_dynamic(&value).map_err(|err| rhai_error(err.to_string()))
}

fn resolve_scoped_path(base_dir: &Path, relative_path: &str, scope_name: &str) -> RhaiResult<PathBuf> {
    let candidate = Path::new(relative_path);
    if candidate.is_absolute() {
        return Err(rhai_error(format!(
            "{} paths must be relative: {}",
            scope_name, relative_path
        )));
    }

    let mut clean_path = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => clean_path.push(part),
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(rhai_error(format!(
                    "{} paths cannot escape their base directory: {}",
                    scope_name, relative_path
                )));
            }
        }
    }

    Ok(base_dir.join(clean_path))
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

fn rhai_error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), Position::NONE).into()
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

    #[test]
    fn exposes_common_helpers_under_hbs_namespace() {
        let engine = create_engine(None);
        let result = engine
            .eval::<String>(r#"hbs::snake_case("User Profile ID") + ":" + hbs::sum([1, 2]).to_string()"#)
            .unwrap();

        assert_eq!(result, "user_profile_id:3");
    }
}