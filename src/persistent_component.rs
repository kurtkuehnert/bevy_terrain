use bevy::ecs::query::{QueryItem, WorldQuery};
use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use bevy::prelude::*;
use bevy::render::{RenderApp, RenderStage, RenderWorld};
use bevy::utils::HashMap;
use std::marker::PhantomData;

#[derive(Component)]
pub struct InitializeComponent<C> {
    marker: PhantomData<fn() -> C>,
}

impl<C> Default for InitializeComponent<C> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

pub type PersistentComponents<C> = HashMap<Entity, C>;

pub trait PersistentComponent: Component {
    /// This filter determines, for which entities the component should be inserted.
    type InsertFilter: WorldQuery;
    type InitializeQuery: WorldQuery;
    type InitializeParam: SystemParam;
    /// Filters the entities with additional constraints.
    type UpdateFilter: WorldQuery;
    /// ECS [`WorldQuery`] to fetch the components to update from.
    type UpdateQuery: WorldQuery;

    /// Prepare
    fn initialize_component(
        item: QueryItem<Self::InitializeQuery>,
        param: &mut SystemParamItem<Self::InitializeParam>,
    ) -> Self;
    /// Extract
    fn update_component(&mut self, item: QueryItem<Self::UpdateQuery>);
}

pub struct PersistentComponentPlugin<C> {
    marker: PhantomData<C>,
}

impl<C> Default for PersistentComponentPlugin<C> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<C: PersistentComponent> Plugin for PersistentComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<PersistentComponents<C>>()
                .add_system_to_stage(RenderStage::Extract, insert_persistent_component::<C>)
                .add_system_to_stage(RenderStage::Extract, update_persistent_component::<C>)
                .add_system_to_stage(RenderStage::Prepare, initialize_persistent_component::<C>);
        }
    }
}

// Todo: consider using commands instead?
pub(crate) fn insert_persistent_component<C: PersistentComponent>(
    mut commands: Commands,
    query: Query<Entity, C::InsertFilter>,
) {
    for entity in query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(InitializeComponent::<C>::default());
    }
}

pub(crate) fn update_persistent_component<C: PersistentComponent>(
    mut render_world: ResMut<RenderWorld>,
    mut query: StaticSystemParam<Query<(Entity, C::UpdateQuery), C::UpdateFilter>>,
) {
    let mut components = render_world.resource_mut::<PersistentComponents<C>>();

    for (entity, item) in query.iter_mut() {
        let component = match components.get_mut(&entity) {
            Some(component) => component,
            None => continue,
        };

        component.update_component(item);
    }
}

pub(crate) fn initialize_persistent_component<C: PersistentComponent>(
    mut components: ResMut<PersistentComponents<C>>,
    param: StaticSystemParam<C::InitializeParam>,
    mut query: StaticSystemParam<Query<(Entity, C::InitializeQuery), With<InitializeComponent<C>>>>,
) {
    let mut param = param.into_inner();

    for (entity, item) in query.iter_mut() {
        components.insert(entity, C::initialize_component(item, &mut param));
    }
}
