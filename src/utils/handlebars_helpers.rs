use convert_case::{Case, Casing};
use handlebars::{
    Context, Handlebars, Helper, HelperDef, JsonRender, RenderContext, RenderError,
    RenderErrorReason, ScopedJson,
};
use std::cmp::Ordering;
use serde_json::Value;

type ValueHelperFn = fn(&Helper<'_>, &Handlebars<'_>, &Context) -> Result<Value, RenderError>;

pub fn register_common_helpers(handlebars: &mut Handlebars<'_>) {
    handlebars.register_helper("all", Box::new(ValueHelper(all_helper)));
    handlebars.register_helper("any", Box::new(ValueHelper(any_helper)));
    handlebars.register_helper("camel_case", Box::new(ValueHelper(camel_case_helper)));
    handlebars.register_helper("default", Box::new(ValueHelper(default_helper)));
    handlebars.register_helper("eq", Box::new(ValueHelper(eq_helper)));
    handlebars.register_helper("gt", Box::new(ValueHelper(gt_helper)));
    handlebars.register_helper("gte", Box::new(ValueHelper(gte_helper)));
    handlebars.register_helper("isdefined", Box::new(ValueHelper(isdefined_helper)));
    handlebars.register_helper("join", Box::new(ValueHelper(join_helper)));
    handlebars.register_helper("json", Box::new(ValueHelper(json_helper)));
    handlebars.register_helper("kebab_case", Box::new(ValueHelper(kebab_case_helper)));
    handlebars.register_helper("lower", Box::new(ValueHelper(lower_helper)));
    handlebars.register_helper("lt", Box::new(ValueHelper(lt_helper)));
    handlebars.register_helper("lte", Box::new(ValueHelper(lte_helper)));
    handlebars.register_helper("neq", Box::new(ValueHelper(neq_helper)));
    handlebars.register_helper("not", Box::new(ValueHelper(not_helper)));
    handlebars.register_helper("pascal_case", Box::new(ValueHelper(pascal_case_helper)));
    handlebars.register_helper("replace", Box::new(ValueHelper(replace_helper)));
    handlebars.register_helper("screaming_snake_case", Box::new(ValueHelper(screaming_snake_case_helper)));
    handlebars.register_helper("snake_case", Box::new(ValueHelper(snake_case_helper)));
    handlebars.register_helper("sub", Box::new(ValueHelper(sub_helper)));
    handlebars.register_helper("sum", Box::new(ValueHelper(sum_helper)));
    handlebars.register_helper("title_case", Box::new(ValueHelper(title_case_helper)));
    handlebars.register_helper("upper", Box::new(ValueHelper(upper_helper)));
}

struct ValueHelper(ValueHelperFn);

impl HelperDef for ValueHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        registry: &'reg Handlebars<'reg>,
        context: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        Ok(ScopedJson::Derived((self.0)(helper, registry, context)?))
    }
}

fn all_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(
        helper.params().iter().all(|param| is_truthy(param.value())),
    ))
}

fn any_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(
        helper.params().iter().any(|param| is_truthy(param.value())),
    ))
}

fn camel_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::Camel)
}

fn default_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    let value = get_param(helper, 0)?;
    let fallback = get_param(helper, 1)?;

    Ok(if is_blank(value) {
        fallback.clone()
    } else {
        value.clone()
    })
}

fn eq_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| left == right)?))
}

fn gt_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| compare_values(left, right) == Some(Ordering::Greater))?))
}

fn gte_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| {
        compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Less)
    })?))
}

fn isdefined_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(
        helper.param(0).is_some_and(|param| !param.is_value_missing()),
    ))
}

fn join_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    let values = get_param(helper, 0)?;
    let separator = helper
        .param(1)
        .map(|value| value_as_string(value.value()))
        .unwrap_or_else(|| ", ".to_string());

    Ok(Value::String(match values {
        Value::Array(items) => items.iter().map(value_as_string).collect::<Vec<_>>().join(&separator),
        value => render_value(value),
    }))
}

fn json_helper(helper: &Helper<'_>, _: &Handlebars<'_>, context: &Context) -> Result<Value, RenderError> {
    let value = helper.param(0).map(|value| value.value()).unwrap_or_else(|| context.data());
    serde_json::to_string(value)
        .map(Value::String)
        .map_err(|err| render_error(err.to_string()))
}

fn kebab_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::Kebab)
}

fn lower_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::String(value_as_string(get_param(helper, 0)?).to_lowercase()))
}

fn lt_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| compare_values(left, right) == Some(Ordering::Less))?))
}

fn lte_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| {
        compare_values(left, right).is_some_and(|ordering| ordering != Ordering::Greater)
    })?))
}

fn neq_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(compare_chain(helper, |left, right| left != right)?))
}

fn not_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::Bool(!is_truthy(get_param(helper, 0)?)))
}

fn pascal_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::Pascal)
}

fn replace_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    let value = value_as_string(get_param(helper, 0)?);
    let from = value_as_string(get_param(helper, 1)?);
    let to = value_as_string(get_param(helper, 2)?);
    Ok(Value::String(value.replace(&from, &to)))
}

fn screaming_snake_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::UpperSnake)
}

fn snake_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::Snake)
}

fn sub_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    let values = helper.params();
    if values.is_empty() {
        return Err(render_error("sub requires at least one parameter"));
    }

    let first = number_value(values[0].value())?;
    let result = if values.len() == 1 {
        -first
    } else {
        values[1..]
            .iter()
            .try_fold(first, |acc, param| -> Result<f64, RenderError> {
                Ok(acc - number_value(param.value())?)
            })?
    };

    Ok(number_json(result))
}

fn sum_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    let values = helper.params();
    if values.iter().any(|param| matches!(param.value(), Value::String(_))) {
        return Ok(Value::String(
            values.iter().map(|param| value_as_string(param.value())).collect(),
        ));
    }

    let result = values
        .iter()
        .try_fold(0.0, |acc, param| -> Result<f64, RenderError> {
            Ok(acc + number_value(param.value())?)
        })?;
    Ok(number_json(result))
}

fn title_case_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    convert_case_helper(helper, Case::Title)
}

fn upper_helper(helper: &Helper<'_>, _: &Handlebars<'_>, _: &Context) -> Result<Value, RenderError> {
    Ok(Value::String(value_as_string(get_param(helper, 0)?).to_uppercase()))
}

fn convert_case_helper(helper: &Helper<'_>, case: Case) -> Result<Value, RenderError> {
    Ok(Value::String(
        value_as_string(get_param(helper, 0)?).to_case(case),
    ))
}

fn compare_chain(
    helper: &Helper<'_>,
    predicate: impl Fn(&Value, &Value) -> bool,
) -> Result<bool, RenderError> {
    let params = helper.params();
    if params.len() < 2 {
        return Err(render_error("comparison helpers require at least two parameters"));
    }

    Ok(params
        .windows(2)
        .all(|pair| predicate(pair[0].value(), pair[1].value())))
}

fn compare_values(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64()?.partial_cmp(&right.as_f64()?),
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        (Value::Bool(left), Value::Bool(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn get_param<'a>(helper: &'a Helper<'_>, index: usize) -> Result<&'a Value, RenderError> {
    helper
        .param(index)
        .map(|value| value.value())
        .ok_or_else(|| render_error(format!("missing helper parameter at index {}", index)))
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

fn number_value(value: &Value) -> Result<f64, RenderError> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| render_error("numeric helper received unsupported number")),
        _ => Err(render_error("numeric helper parameters must be numbers")),
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