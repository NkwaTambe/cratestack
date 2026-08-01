//! SQL-fragment computation for sorting through to-one relations.
//! `collect_relation_order_targets` walks the model graph collecting
//! every `(api_key, sql_fragment)` pair reachable through to-one
//! relations, for the REST `?orderBy=` string-key match arms.
//!
//! NOTE: this walk is itself path-enumerating and therefore exponential in
//! to-one connectivity — the same shape as the codegen bug fixed in #252,
//! but far cheaper (it emits strings, not modules, and follows only to-one
//! edges). Left as-is deliberately; it deserves its own issue.

use cratestack_core::Model;

use crate::shared::{
    find_model, model_name_set, relation_model_fields, scalar_model_fields, to_snake_case,
};

use super::types::{relation_link, relation_visit_key};

pub(crate) fn collect_relation_order_targets(
    model: &Model,
    models: &[Model],
    current_table: &str,
    prefix: &str,
) -> Result<Vec<(String, String)>, String> {
    collect_inner(model, models, current_table, prefix, &[])
}

fn collect_inner(
    model: &Model,
    models: &[Model],
    current_table: &str,
    prefix: &str,
    visited: &[String],
) -> Result<Vec<(String, String)>, String> {
    let model_names = model_name_set(models);
    let mut targets = scalar_model_fields(model, &model_names)
        .into_iter()
        .map(|field| {
            (
                format!("{}.{}", prefix, field.name),
                format!("{}.{}", current_table, to_snake_case(&field.name)),
            )
        })
        .collect::<Vec<_>>();

    for relation_field in relation_model_fields(model, &model_names) {
        let visit_key = relation_visit_key(model, relation_field);
        if visited.contains(&visit_key) {
            continue;
        }
        let relation_link = relation_link(model, relation_field, models)?;
        if relation_link.is_to_many {
            continue;
        }
        let target_model = find_model(models, &relation_field.ty.name).ok_or_else(|| {
            format!(
                "relation field `{}` on `{}` references unknown model `{}`",
                relation_field.name, model.name, relation_field.ty.name,
            )
        })?;
        let mut next_visited = visited.to_vec();
        next_visited.push(visit_key);
        let nested_targets = collect_inner(
            target_model,
            models,
            relation_link.related_table.as_str(),
            &format!("{}.{}", prefix, relation_field.name),
            &next_visited,
        )?;
        targets.extend(nested_targets.into_iter().map(|(key, nested_sql)| {
            (
                key,
                format!(
                    "(SELECT {} FROM {} WHERE {}.{} = {}.{} LIMIT 1)",
                    nested_sql,
                    relation_link.related_table,
                    relation_link.related_table,
                    relation_link.related_column,
                    current_table,
                    relation_link.parent_column,
                ),
            )
        }));
    }

    Ok(targets)
}
