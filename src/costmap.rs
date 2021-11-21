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
            .add_startup_system(setup.system())
            .add_system(update.system());
    }
}
const COSTMAP_SIZE: usize = 10;
const COSTMAP_RESOLUTION: f32 = 10.0;
const UNOCCUPIED_COST: Cost = 0;
const OCCUPIED_COST: Cost = 1;

pub type SharedCostmap = Costmap<COSTMAP_SIZE,COSTMAP_SIZE>;

fn setup(
    mut commands: Commands,
) {

    commands.insert_resource(SharedCostmap::new(
        Mat3::from_scale_angle_translation(
            Vec2::splat(COSTMAP_RESOLUTION),
            0.0,
            Vec2::ZERO)
    ));
}

fn update(
    mut costmap: ResMut<SharedCostmap>,
    q: Query<(&InteractionGroups, &RigidBodyPosition, &ColliderShape)>
) {
    for (ig, rb_pos, shape) in q.iter() {
        costmap.set_cost(UNOCCUPIED_COST, ig, shape, &rb_pos.position);
        costmap.set_cost(OCCUPIED_COST, ig, shape, &rb_pos.position);
    }
}

fn draw_grid(
    mut commands: Commands,
    rc: Res<RapierConfiguration>,
    grid: Res<SharedCostmap>
) {
    for (x, y) in grid.ititerer() {
        commands.spawn_bundle(GeometryBuilder::build_as(
            &shapes::Circle {
                radius: 1.0,
                center: Vec2::ZERO,
            },
            ShapeColors::new(Color::BLUE),
            DrawMode::Fill(FillOptions::default()),
            Transform::from_translation(Vec3::new(rc.scale * x as f32, rc.scale * y as f32, 10.0))
        ));
    }
}

pub type Cost = u8;

#[derive(Clone, Copy)]
pub struct CostmapCell {
    cost: Cost,
    interaction_groups: InteractionGroups
}

impl Default for CostmapCell {
    fn default() -> Self {
        CostmapCell {
            cost: 0,
            interaction_groups: InteractionGroups::all()
        }
    }
}
pub struct Costmap<const M: usize, const N: usize> {
    transform: Mat3,
    data: [[CostmapCell; N]; M]
}

pub struct CostmapIterator<const M: usize, const N: usize> {
    costmap: Costmap<M,N>,
    index: usize
}

impl<const M: usize, const N: usize> IntoIterator for Costmap<M,N> {
    type Item = CostmapCell;
    type IntoIter = CostmapIterator<M,N>;

    fn into_iter(self) -> Self::IntoIter {
        CostmapIterator {
            costmap: self,
            index: 0
        }
    }
}

impl<const M: usize, const N: usize> Iterator for CostmapIterator<M,N> {
    type Item = CostmapCell;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.index / M;
        let column = self.index - (row * N);

        self.index += 1;

        if row >= M || column >= N {
            None
        } else {
            Some(self.costmap.data[row][column])
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

            for x in x_min..x_max {
                for y in y_min..y_max {
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