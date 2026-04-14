use convert_case::{Case, Casing};
use handlebars::{
    Context, Handlebars, Helper, JsonRender, Output, RenderContext, RenderError,
    RenderErrorReason,
};
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::Value;
use std::{any::TypeId, cmp::Ordering, collections::HashMap, sync::LazyLock};

type HelperFn = fn(&[Value], Option<&Value>) -> Result<Value, String>;
type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

static HELPERS: LazyLock<HashMap<&'static str, HelperFn>> = LazyLock::new(|| {
    HashMap::from([
        ("all", all as HelperFn),
        ("any", any as HelperFn),
        ("camel_case", camel_case as HelperFn),
        ("default", default as HelperFn),
        ("eq", eq as HelperFn),
        ("gt", gt as HelperFn),
        ("gte", gte as HelperFn),
        ("isdefined", isdefined as HelperFn),
        ("join", join as HelperFn),
        ("json", json as HelperFn),
        ("kebab_case", kebab_case as HelperFn),
        ("lower", lower as HelperFn),
        ("lt", lt as HelperFn),
        ("lte", lte as HelperFn),
        ("neq", neq as HelperFn),
        ("not", not as HelperFn),
        ("pascal_case", pascal_case as HelperFn),
        ("replace", replace as HelperFn),
        ("screaming_snake_case", screaming_snake_case as HelperFn),
        ("snake_case", snake_case as HelperFn),
        ("sub", sub as HelperFn),
        ("sum", sum as HelperFn),
        ("title_case", title_case as HelperFn),
        ("upper", upper as HelperFn),
        ("contains", contains as HelperFn),
    ])
});

pub fn register_handlebars_helpers(handlebars: &mut Handlebars<'_>) {
    for (&name, &helper) in HELPERS.iter() {
        handlebars.register_helper(
            name,
            Box::new(
                move |
                    h: &Helper<'_>,
                    _: &Handlebars<'_>,
                    ctx: &Context,
                    _: &mut RenderContext<'_, '_>,
                    out: &mut dyn Output,
                | {
                    let args = h.params().iter().map(|param| param.value().clone()).collect::<Vec<_>>();
                    let value = helper(&args, Some(ctx.data())).map_err(render_error)?;
                    out.write(&render_value(&value)).map_err(|err| render_error(err.to_string()))?;
                    Ok(())
                },
            ),
        );
    }
}

pub fn register_handlebars_rhai_module(engine: &mut Engine) {
    for (&name, &helper) in HELPERS.iter() {
        for arity in 0..=8 {
            engine.register_raw_fn(
                name,
                vec![TypeId::of::<Dynamic>(); arity],
                move |_, args| -> RhaiResult<Dynamic> {
                    let args = args
                        .iter()
                        .map(|arg| {
                            rhai::serde::from_dynamic(arg)
                                .map_err(|err| rhai_error(err.to_string()))
                        })
                        .collect::<RhaiResult<Vec<Value>>>()?;

                    let value = helper(&args, None).map_err(rhai_error)?;
                    rhai::serde::to_dynamic(value).map_err(|err| rhai_error(err.to_string()))
                },
            );
        }
    }
}

fn all(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(args.iter().all(is_truthy)))
}

fn any(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(args.iter().any(is_truthy)))
}

fn camel_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::Camel)
}

fn default(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(if is_blank(arg(args, 0)?) {
        arg(args, 1)?.clone()
    } else {
        arg(args, 0)?.clone()
    })
}

fn eq(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| left == right)?))
}

fn gt(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| {
        compare_values(left, right) == Some(Ordering::Greater)
    })?))
}

fn gte(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| {
        compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Less)
    })?))
}

fn isdefined(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(!args.is_empty()))
}

fn join(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    let separator = args.get(1).map(string).unwrap_or_else(|| ", ".to_string());

    Ok(Value::String(match arg(args, 0)? {
        Value::Array(items) => items.iter().map(string).collect::<Vec<_>>().join(&separator),
        value => render_value(value),
    }))
}

fn json(args: &[Value], root: Option<&Value>) -> Result<Value, String> {
    serde_json::to_string(args.first().or(root).unwrap_or(&Value::Null))
        .map(Value::String)
        .map_err(|err| err.to_string())
}

fn kebab_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::Kebab)
}

fn lower(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::String(string(arg(args, 0)?).to_lowercase()))
}

fn lt(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| {
        compare_values(left, right) == Some(Ordering::Less)
    })?))
}

fn lte(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| {
        compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Greater)
    })?))
}

fn neq(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(compare_chain(args, |left, right| left != right)?))
}

fn not(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::Bool(!is_truthy(arg(args, 0)?)))
}

fn pascal_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::Pascal)
}

fn replace(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::String(
        string(arg(args, 0)?).replace(&string(arg(args, 1)?), &string(arg(args, 2)?)),
    ))
}

fn screaming_snake_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::UpperSnake)
}

fn snake_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::Snake)
}

fn sub(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("sub requires at least one parameter".to_string());
    }

    let first = number(&args[0])?;
    let result = if args.len() == 1 {
        -first
    } else {
        args[1..]
            .iter()
            .try_fold(first, |acc, value| -> Result<f64, String> {
                Ok(acc - number(value)?)
            })?
    };

    Ok(number_json(result))
}

fn sum(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    if args.iter().any(|value| matches!(value, Value::String(_))) {
        return Ok(Value::String(args.iter().map(string).collect()));
    }

    Ok(number_json(
        args.iter()
            .try_fold(0.0, |acc, value| -> Result<f64, String> {
                Ok(acc + number(value)?)
            })?,
    ))
}

fn title_case(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    convert_case(args, Case::Title)
}

fn upper(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    Ok(Value::String(string(arg(args, 0)?).to_uppercase()))
}

fn convert_case(args: &[Value], case: Case) -> Result<Value, String> {
    Ok(Value::String(string(arg(args, 0)?).to_case(case)))
}

fn compare_chain(args: &[Value], predicate: impl Fn(&Value, &Value) -> bool) -> Result<bool, String> {
    if args.len() < 2 {
        return Err("comparison helpers require at least two parameters".to_string());
    }

    Ok(args.windows(2).all(|pair| predicate(&pair[0], &pair[1])))
}

fn compare_values(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64()?.partial_cmp(&right.as_f64()?),
        (Value::String(left), Value::String(right)) => 
            match (left.parse::<f64>(), right.parse::<f64>()) {
                (Ok(l), Ok(r)) => l.partial_cmp(&r),
                _ => Some(left.cmp(right)),
            }
        (Value::String(l), Value::Number(r)) => l.parse::<f64>().ok()?.partial_cmp(&r.as_f64()?),
        (Value::Number(l), Value::String(r)) => l.as_f64()?.partial_cmp(&r.parse::<f64>().ok()?),
        (Value::Bool(left), Value::Bool(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn contains(args: &[Value], _: Option<&Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("contains expects exactly 2 arguments".into());
    }

    let haystack = &args[0];
    let needle = &args[1];

    let result = match (haystack, needle) {
        (Value::Array(arr), _) => arr.iter().any(|v| v == needle),
        (Value::String(s), Value::String(sub)) => s.contains(sub),
        (Value::Object(map), Value::String(key)) => map.contains_key(key),
        _ => false,
    };

    Ok(Value::Bool(result))
}

fn arg(args: &[Value], index: usize) -> Result<&Value, String> {
    args.get(index)
        .ok_or_else(|| format!("missing helper parameter at index {index}"))
}

fn string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        _ => value.render(),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        Value::Bool(b) => {
            if *b { "true".into() } else { "".into() }
        }
        _ => string(value),
    }
}

fn number(value: &Value) -> Result<f64, String> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| "numeric helper received unsupported number".to_string()),
        _ => Err("numeric helper parameters must be numbers".to_string()),
    }
}

fn number_json(value: f64) -> Value {
    if value.fract() == 0.0 {
        Value::from(value as i64)
    } else {
        Value::from(value)
    }
}

fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        _ => false,
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
    }
}

fn render_error(message: impl Into<String>) -> RenderError {
    RenderErrorReason::Other(message.into()).into()
}

fn rhai_error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), rhai::Position::NONE).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_handlebars_helpers() {
        let mut handlebars = Handlebars::new();
        register_handlebars_helpers(&mut handlebars);

        let rendered = handlebars
            .render_template(
                "{{lower name}}|{{upper name}}|{{join tags \";\"}}|{{default fallback \"n/a\"}}|{{replace name \"A\" \"a\"}}|{{sum one two}}|{{sub two one}}|{{pascal_case phrase}}|{{camel_case phrase}}|{{snake_case phrase}}|{{kebab_case phrase}}|{{screaming_snake_case phrase}}|{{title_case phrase}}",
                &serde_json::json!({
                    "name": "Ada",
                    "tags": ["x", "y"],
                    "fallback": null,
                    "one": 1,
                    "phrase": "user profile ID",
                    "two": 2,
                }),
            )
            .unwrap();

        assert_eq!(rendered, "ada|ADA|x;y|n/a|ada|3|1|UserProfileId|userProfileId|user_profile_id|user-profile-id|USER_PROFILE_ID|User Profile Id");
    }

    #[test]
    fn supports_boolean_subexpressions() {
        let mut handlebars = Handlebars::new();
        register_handlebars_helpers(&mut handlebars);

        let rendered = handlebars
            .render_template(
                "{{#if (all (eq x 3) (gt y x) (isdefined z) (not empty))}}ok{{else}}no{{/if}}|{{#if (neq x 4 5)}}yes{{else}}no{{/if}}|{{#if (lte 1 x y 5)}}in{{else}}out{{/if}}",
                &serde_json::json!({
                    "x": 3,
                    "y": 4,
                    "z": true,
                    "empty": false,
                }),
            )
            .unwrap();

        assert_eq!(rendered, "ok|yes|in");
    }

    #[test]
    fn registers_rhai_helpers() {
        let mut engine = Engine::new();
        register_handlebars_rhai_module(&mut engine);

        assert_eq!(engine.eval::<i64>("sum(1, 2, 3)").unwrap(), 6);
        assert_eq!(
            engine.eval::<String>(r#"snake_case("user profile ID")"#).unwrap(),
            "user_profile_id"
        );
        assert!(engine.eval::<bool>("all(true, 1, \"x\")").unwrap());
    }
}
