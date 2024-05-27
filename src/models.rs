use crate::api::ListModels;

pub fn installed_models() -> Vec<String> {
    let mut models: Vec<String> = Vec::new();

    let tags = ListModels::new();
    if let Ok(response) = tags.result {
        response.models.iter().for_each(|field| {
            models.push(field.model.to_owned());
        });
    }

    models
}
