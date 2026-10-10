use std::{fs, time::{Duration, Instant, SystemTime}};

use crate::{shader::ShaderInput, utility::{PipeLineType, UniformType}};

const CHECK_INTERVAL: Duration = Duration::from_millis(300);

#[derive(Clone)]
pub(crate) struct WatchedFile
{
    pub(crate) path: String,
    modified: SystemTime
}

#[derive(Clone)]
pub(crate) struct ShaderRecipe
{
    pub(crate) pipeline_id: usize,
    pub(crate) vertex: Option<WatchedFile>,
    pub(crate) fragment: Option<WatchedFile>,
    pub(crate) pipeline_type: PipeLineType,
    pub(crate) uniforms: Option<Vec<(String, UniformType)>>
}

pub(crate) struct ShaderWatcher
{
    recipes: Vec<ShaderRecipe>,
    last_check: Instant
}

impl ShaderWatcher
{
    pub(crate) fn new() -> Self
    {
        Self { recipes: Vec::new(), last_check: Instant::now() }
    }

    pub(crate) fn track(&mut self, pipeline_id: usize, vertex: Option<ShaderInput>, fragment: Option<ShaderInput>, pipeline_type: PipeLineType, uniforms: Option<&[(&str, UniformType)]>)
    {
        self.recipes.retain(|recipe| recipe.pipeline_id != pipeline_id);

        let recipe = ShaderRecipe
        {
            pipeline_id,
            vertex: watch(vertex),
            fragment: watch(fragment),
            pipeline_type,
            uniforms: uniforms.map(|list| list.iter().map(|&(name, kind)| (name.to_string(), kind)).collect())
        };

        if recipe.vertex.is_some() || recipe.fragment.is_some()
        {
            self.recipes.push(recipe);
        }
    }

    pub(crate) fn changed(&mut self) -> Vec<ShaderRecipe>
    {
        if self.last_check.elapsed() < CHECK_INTERVAL
        {
            return Vec::new();
        }
        self.last_check = Instant::now();

        let mut changed = Vec::new();
        for recipe in &mut self.recipes
        {
            if check(&mut recipe.vertex) | check(&mut recipe.fragment) // | to check both (gives timestamp)
            {
                changed.push(recipe.clone());
            }
        }
        changed
    }
}

fn modified_time(path: &str) -> Option<SystemTime>
{
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn watch(input: Option<ShaderInput>) -> Option<WatchedFile>
{
    match input
    {
        Some(ShaderInput::File(path)) => modified_time(path).map(|modified| WatchedFile { path: path.to_string(), modified }),
        _ => None
    }
}

fn check(file: &mut Option<WatchedFile>) -> bool
{
    let Some(file) = file else { return false; };

    match modified_time(&file.path)
    {
        Some(time) if time != file.modified =>
        {
            file.modified = time;
            true
        }
        _ => false
    }
}
