use convert_case::{Case, Casing};
use handlebars::{
    Context, Handlebars, Helper, HelperDef, JsonRender, RenderContext, RenderError,
    RenderErrorReason, ScopedJson,
};
use serde_json::Value;
use std::cmp::Ordering;

const COMMON_HELPER_NAMES: &[&str] = &[
    "all",
    "any",
    "camel_case",
    "default",
    "eq",
    "gt",
    "gte",
    "isdefined",
    "join",
    "json",
    "kebab_case",
    "lower",
    "lt",
    "lte",
    "neq",
    "not",
    "pascal_case",
    "replace",
    "screaming_snake_case",
    "snake_case",
    "sub",
    "sum",
    "title_case",
    "upper",
];

pub fn common_helper_names() -> &'static [&'static str] {
    COMMON_HELPER_NAMES
}

pub fn register_common_helpers(handlebars: &mut Handlebars<'_>) {
    for helper_name in common_helper_names() {
        handlebars.register_helper(helper_name, Box::new(ValueHelper { name: helper_name }));
    }
}

pub fn evaluate_common_helper(
    helper_name: &str,
    args: &[Value],
    root: Option<&Value>,
) -> Result<Value, String> {
    match helper_name {
        "all" => Ok(Value::Bool(args.iter().all(is_truthy))),
        "any" => Ok(Value::Bool(args.iter().any(is_truthy))),
        "camel_case" => convert_case_value(args, Case::Camel),
        "default" => Ok(if is_blank(get_arg(args, 0)?) {
            get_arg(args, 1)?.clone()
        } else {
            get_arg(args, 0)?.clone()
        }),
        "eq" => Ok(Value::Bool(compare_chain(args, |left, right| left == right)?)),
        "gt" => Ok(Value::Bool(compare_chain(args, |left, right| {
            compare_values(left, right) == Some(Ordering::Greater)
        })?)),
        "gte" => Ok(Value::Bool(compare_chain(args, |left, right| {
            compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Less)
        })?)),
        "isdefined" => Ok(Value::Bool(!args.is_empty())),
        "join" => join_value(args),
        "json" => json_value(args, root),
        "kebab_case" => convert_case_value(args, Case::Kebab),
        "lower" => Ok(Value::String(value_as_string(get_arg(args, 0)?).to_lowercase())),
        "lt" => Ok(Value::Bool(compare_chain(args, |left, right| {
            compare_values(left, right) == Some(Ordering::Less)
        })?)),
        "lte" => Ok(Value::Bool(compare_chain(args, |left, right| {
            compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Greater)
        })?)),
        "neq" => Ok(Value::Bool(compare_chain(args, |left, right| left != right)?)),
        "not" => Ok(Value::Bool(!is_truthy(get_arg(args, 0)?))),
        "pascal_case" => convert_case_value(args, Case::Pascal),
        "replace" => Ok(Value::String(
            value_as_string(get_arg(args, 0)?)
                .replace(&value_as_string(get_arg(args, 1)?), &value_as_string(get_arg(args, 2)?)),
        )),
        "screaming_snake_case" => convert_case_value(args, Case::UpperSnake),
        "snake_case" => convert_case_value(args, Case::Snake),
        "sub" => sub_value(args),
        "sum" => sum_value(args),
        "title_case" => convert_case_value(args, Case::Title),
        "upper" => Ok(Value::String(value_as_string(get_arg(args, 0)?).to_uppercase())),
        _ => Err(format!("unknown helper '{}'", helper_name)),
    }
}

struct ValueHelper {
    name: &'static str,
}

impl HelperDef for ValueHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        registry: &'reg Handlebars<'reg>,
        context: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let _ = registry;
        let args = helper
            .params()
            .iter()
            .map(|param| param.value().clone())
            .collect::<Vec<_>>();

        let value = evaluate_common_helper(self.name, &args, Some(context.data()))
            .map_err(render_error)?;
        Ok(ScopedJson::Derived(value))
    }
}

fn convert_case_value(args: &[Value], case: Case) -> Result<Value, String> {
    Ok(Value::String(value_as_string(get_arg(args, 0)?).to_case(case)))
}

fn join_value(args: &[Value]) -> Result<Value, String> {
    let values = get_arg(args, 0)?;
    let separator = args
        .get(1)
        .map(value_as_string)
        .unwrap_or_else(|| ", ".to_string());

    Ok(Value::String(match values {
        Value::Array(items) => items.iter().map(value_as_string).collect::<Vec<_>>().join(&separator),
        value => render_value(value),
    }))
}

fn json_value(args: &[Value], root: Option<&Value>) -> Result<Value, String> {
    let value = args.first().or(root).unwrap_or(&Value::Null);
    serde_json::to_string(value)
        .map(Value::String)
        .map_err(|err| err.to_string())
}

fn sub_value(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("sub requires at least one parameter".to_string());
    }

    let first = number_value(&args[0])?;
    let result = if args.len() == 1 {
        -first
    } else {
        args[1..]
            .iter()
            .try_fold(first, |acc, value| -> Result<f64, String> {
                Ok(acc - number_value(value)?)
            })?
    };

    Ok(number_json(result))
}

fn sum_value(args: &[Value]) -> Result<Value, String> {
    if args.iter().any(|value| matches!(value, Value::String(_))) {
        return Ok(Value::String(args.iter().map(value_as_string).collect()));
    }

    let result = args
        .iter()
        .try_fold(0.0, |acc, value| -> Result<f64, String> {
            Ok(acc + number_value(value)?)
        })?;
    Ok(number_json(result))
}

fn compare_chain(
    values: &[Value],
    predicate: impl Fn(&Value, &Value) -> bool,
) -> Result<bool, String> {
    if values.len() < 2 {
        return Err("comparison helpers require at least two parameters".to_string());
    }

    Ok(values
        .windows(2)
        .all(|pair| predicate(&pair[0], &pair[1])))
}

fn compare_values(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64()?.partial_cmp(&right.as_f64()?),
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        (Value::Bool(left), Value::Bool(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn get_arg(values: &[Value], index: usize) -> Result<&Value, String> {
    values
        .get(index)
        .ok_or_else(|| format!("missing helper parameter at index {}", index))
}

fn value_as_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        _ => value.render(),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        _ => value_as_string(value),
    }
}

fn number_value(value: &Value) -> Result<f64, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_common_helpers() {
        let mut handlebars = Handlebars::new();
        register_common_helpers(&mut handlebars);

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
        register_common_helpers(&mut handlebars);

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
}