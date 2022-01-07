use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use bevy_prototype_lyon::prelude::*;

pub struct CostmapPlugin;

impl Plugin for CostmapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_startup_system_to_stage(StartupStage::PreStartup, setup.system().after("setup_physics"))  // For some reason it panicks if it runs later
            .add_system_to_stage(CoreStage::PreUpdate, reset_costmap.system().label("reset_costmap"))
            .add_system_to_stage(CoreStage::PreUpdate, update.system().label("update_costmap").after("reset_costmap"));
    }
}

const COSTMAP_SIZE: usize = 40; // number of cells in each dimension (this squared for total)
const COSTMAP_RESOLUTION: f32 = 0.25; // meters per costmap cell

const OCCUPIED_COLOR: Color = Color::rgba(1.0, 0.0, 0.0, 0.5);
const UNOCCUPIED_COLOR: Color = Color::rgba(0.0, 0.0, 1.0, 0.5);

pub type SharedCostmap = Costmap<COSTMAP_SIZE,COSTMAP_SIZE>;

pub struct CostmapCellCoordinates {
    coordinates: (usize, usize)
}


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

    for row in 0..costmap.data.len() {
        for column in 0..costmap.data[0].len() {
            let physics_position = costmap.to_physics_position(row, column);
            let pixel_position = rc.scale * physics_position;
            let pixels_per_box = COSTMAP_RESOLUTION * rc.scale;
            commands.spawn_bundle(GeometryBuilder::build_as(
                &shapes::Rectangle {
                    width: pixels_per_box,
                    height: pixels_per_box,
                    origin: shapes::RectangleOrigin::Center
                },
                ShapeColors::new(UNOCCUPIED_COLOR),
                DrawMode::Fill(FillOptions::default()),
                Transform::from_translation(Vec3::new(pixel_position.x, pixel_position.y, 10.0))
            ))
            .insert(CostmapCellCoordinates { coordinates: (row, column) });
        }
    }

    commands.insert_resource(costmap);

    commands.insert_resource(CostmapResetTimer(Timer::from_seconds(0.2, true)));
}

fn reset_costmap(
    mut meshes: ResMut<Assets<Mesh>>,
    mut costmap: ResMut<SharedCostmap>,
    time: Res<Time>,
    mut timer: ResMut<CostmapResetTimer>,
    mut viz_query: Query<(&CostmapCellCoordinates, &Handle<Mesh>)>
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        for mut element in costmap.data.iter_mut().flat_map(|r| r.iter_mut()) {
            element.cost = Cost::UNOCCUPIED;
        }
        for (_, mesh_handle) in viz_query.iter_mut() {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                let color_attribute = <[f32; 4]>::from(UNOCCUPIED_COLOR);
                mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, vec![
                    color_attribute.clone(); mesh.count_vertices()
                ]);
            }
        }
    }
}

fn update(
    mut meshes: ResMut<Assets<Mesh>>,
    mut costmap: ResMut<SharedCostmap>,
    q: Query<(&ColliderFlags, &RigidBodyPosition, &ColliderShape)>,
    mut viz_query: Query<(&CostmapCellCoordinates, &Handle<Mesh>)>
) {
    for (i, (ColliderFlags { collision_groups: ig, .. }, rb_pos, shape)) in q.iter().enumerate() {
        let occupied_cells = costmap.set_cost(Cost::OCCUPIED, ig, shape, &rb_pos.position);
        for (CostmapCellCoordinates { coordinates }, mesh_handle) in viz_query.iter_mut() {
            if occupied_cells.contains(coordinates) {
                if let Some(mesh) = meshes.get_mut(mesh_handle) {
                    let color_attribute = <[f32; 4]>::from(OCCUPIED_COLOR);
                    mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, vec![
                        color_attribute.clone(); mesh.count_vertices()
                    ]);
                }
            }
        }
    }
}

struct CostmapResetTimer(Timer);


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

impl<const M: usize, const N: usize> Costmap<M,N> {
    fn new(transform: Mat3) -> Self {
        Costmap::<M,N> {
            transform,
            ..Default::default()
        }
    }

    fn to_row_column(&self, physics_position: Vec2) -> (usize, usize) {
        let transformed = (self.transform.transform_vector2(physics_position)).round();
        (transformed.x as usize, transformed.y as usize)
    }

    fn to_physics_position(&self, row: usize, column: usize) -> Vec2 {
        self.transform.inverse().transform_vector2(Vec2::new(row as f32, column as f32))
    }

    fn set_cost(
        &mut self,
        cost: Cost,
        interaction_groups: &InteractionGroups,
        shape: &SharedShape,
        pos: &Isometry2<f32>) -> Vec<(usize, usize)> {
            let aabb = shape.compute_aabb(pos);

            let corner1 = self.to_row_column(aabb.mins.into());
            let corner2 = self.to_row_column(aabb.maxs.into());

            let min_row = std::cmp::min(corner1.0, corner2.0);
            let max_row = std::cmp::max(corner1.0, corner2.0);

            let min_column = std::cmp::min(corner1.1, corner2.1);
            let max_column = std::cmp::max(corner1.1, corner2.1);


            let mut costmap_cell_coordinates = Vec::new();
            costmap_cell_coordinates.reserve((max_row - min_row) * (max_column - min_column));

            for row in min_row..=max_row {
                for column in min_column..=max_column {
                    // cell.interaction_groups = InteractionGroups::new(
                    //     cell.interaction_groups.memberships | interaction_groups.memberships,
                    //     cell.interaction_groups.filter | interaction_groups.filter
                    // );
                    self.data[row][column].cost = cost;
                    costmap_cell_coordinates.push((row,column));
                }
            }
                    // let (row, column) = self.to_row_column(pos.translation.into());
                    // self.data[row][column].cost = cost;
            costmap_cell_coordinates
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
