use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use std::ops::Index;
use std::ops::IndexMut;
use bevy_prototype_lyon::prelude::*;


pub struct CostmapPlugin;

impl Plugin for CostmapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_startup_system_to_stage(StartupStage::PreStartup, setup.system()) // For some reason it panicks if it runs later
            .add_system(update.system())
            .add_system(update_grid_viz.system());
    }
}

const COSTMAP_SIZE: usize = 50;
const COSTMAP_RESOLUTION: f32 = 10.0; // meters per costmap cell?

pub type SharedCostmap = Costmap<COSTMAP_SIZE,COSTMAP_SIZE>;

pub struct CostmapCellVisualizationTag;

fn setup(
    mut commands: Commands,
    rc: Res<RapierConfiguration>
) {
    let costmap = SharedCostmap::new(
        Mat3::from_scale_angle_translation(
            Vec2::splat(1.0 / COSTMAP_RESOLUTION),
            std::f32::consts::FRAC_PI_2,
            Vec2::ZERO)
    );

    for (pos, CostmapCell { cost , .. }) in costmap.iter() {
        commands.spawn_bundle(GeometryBuilder::build_as(
            &shapes::Circle {
                radius: 1.0,
                center: Vec2::ZERO,
            },
            ShapeColors::new(match cost {
                Cost::UNOCCUPIED => Color::BLUE,
                Cost::OCCUPIED => Color::RED
            }),
            DrawMode::Fill(FillOptions::default()),
            Transform::from_translation(Vec3::new(rc.scale * pos.x, rc.scale * pos.y, 10.0))
        ))
        .insert(CostmapCellVisualizationTag);
    }

    commands.insert_resource(costmap);

    commands.insert_resource(VisualizationUpdateTimer(Timer::from_seconds(1.0, true)));
}


fn update(
    mut costmap: ResMut<SharedCostmap>,
    q: Query<(&ColliderFlags, &RigidBodyPosition, &ColliderShape)>
) {
    for (ColliderFlags { collision_groups: ig, .. }, rb_pos, shape) in q.iter() {
        costmap.set_cost(Cost::OCCUPIED, ig, shape, &rb_pos.position);
        costmap.set_cost(Cost::OCCUPIED, ig, shape, &rb_pos.next_position);
    }
}

struct VisualizationUpdateTimer(Timer);

fn update_grid_viz(
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mut timer: ResMut<VisualizationUpdateTimer>,
    costmap: Res<SharedCostmap>,
    rc: Res<RapierConfiguration>,
    mut q: Query<(&Transform, &Handle<Mesh>), With<CostmapCellVisualizationTag>>
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        for (transform, mesh_handle) in q.iter_mut() {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                let cell = &costmap[Vec2::from(transform.translation) / rc.scale];
                let color_attribute = <[f32; 4]>::from(
                    match cell.cost {
                        Cost::UNOCCUPIED => Color::BLUE,
                        Cost::OCCUPIED => Color::RED
                    }
                );
                mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, vec![
                    color_attribute.clone(); mesh.count_vertices()
                ]);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Cost {
    UNOCCUPIED,
    OCCUPIED
}

#[derive(Clone, Copy)]
pub struct CostmapCell {
    cost: Cost,
    interaction_groups: InteractionGroups
}

impl Default for CostmapCell {
    fn default() -> Self {
        CostmapCell {
            cost: Cost::UNOCCUPIED,
            interaction_groups: InteractionGroups::all()
        }
    }
}
pub struct Costmap<const M: usize, const N: usize> {
    transform: Mat3,
    data: [[CostmapCell; N]; M]
}

pub struct CostmapIterator<'a, const M: usize, const N: usize> {
    costmap: &'a Costmap<M,N>,
    index: usize
}

impl<'a, const M: usize, const N: usize> Iterator for CostmapIterator<'a, M,N> {
    type Item = (Vec2, CostmapCell);

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.index / M;
        let column = self.index - (row * N);

        self.index += 1;

        if row >= M || column >= N {
            None
        } else {
            Some((
                self.costmap.transform.inverse().transform_vector2(Vec2::new(row as f32, column as f32)),
                self.costmap.data[row][column]
            ))
        }
    }
}

impl<const M: usize, const N: usize> Costmap<M,N> {
    fn new(transform: Mat3) -> Self {
        Costmap::<M,N> {
            transform,
            ..Default::default()
        }
    }

    fn iter(&self) -> CostmapIterator<M,N> {
        CostmapIterator {
            costmap: &self,
            index: 0
        }
    }

    fn set_cost(
        &mut self,
        cost: Cost,
        interaction_groups: &InteractionGroups,
        shape: &SharedShape,
        pos: &Isometry2<f32>) {
            let aabb = shape.compute_aabb(pos);

            let vec_min: Vec2 = aabb.mins.into();
            let vec_max: Vec2 = aabb.maxs.into();

            let min = vec_min.floor();
            let x_min = min.x as usize;
            let y_min = min.y as usize;

            let max = vec_max.ceil();
            let x_max = max.x as usize;
            let y_max = max.y as usize;

            for x in x_min..=x_max {
                for y in y_min..=y_max {
                    let cell = &mut self.data[x][y];
                    cell.interaction_groups = InteractionGroups::new(
                        cell.interaction_groups.memberships | interaction_groups.memberships,
                        cell.interaction_groups.filter | interaction_groups.filter
                    );
                    cell.cost = cost;
                }
            }
        }
}

impl<const M: usize, const N: usize> Default for Costmap<M,N> {
    fn default() -> Self {
        Costmap::<M,N> {
            transform: Mat3::IDENTITY,
            data: [[CostmapCell::default(); N]; M]
        }
    }
}

impl<const M: usize, const N: usize> Index<Vec2> for Costmap<M,N> {
    type Output = CostmapCell;
    fn index(&self, vec2: Vec2) -> &CostmapCell {
        let scaled_rounded = (self.transform.transform_vector2(vec2)).round();
        &self.data[scaled_rounded.x as usize][scaled_rounded.y as usize]
    }
}

impl<const M: usize, const N: usize> IndexMut<Vec2> for Costmap<M,N> {
    fn index_mut(&mut self, vec2: Vec2) -> &mut CostmapCell {
        let transformed = (self.transform.transform_vector2(vec2)).round();
        &mut self.data[transformed.x as usize][transformed.y as usize]
    }
}