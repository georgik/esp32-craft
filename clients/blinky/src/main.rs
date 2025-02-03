extern crate valence;
use bevy::prelude::TimerMode;
use valence::prelude::*;

use std::fs;
use valence::entity::chicken::ChickenEntityBundle;
use valence::entity::Velocity;
use rand::Rng;
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::message::ChatMessageEvent;
use valence::protocol::packets::play::{ChatMessageC2s, ChatMessageS2c};

// Blink
use valence::command_macros::Command;
use bevy::time::{Time, Timer};
use bevy::time::TimePlugin;
use valence::command::AddCommand;
use valence::command::handler::CommandResultEvent;

#[derive(Component)]
struct Blinking {
    is_active: bool,
    timer: Timer,
}



const SPAWN_Y: i32 = 64;
const CHEST_POS: [i32; 3] = [0, SPAWN_Y + 1, 3];
const REDSTONE_LAMP_POS: [i32; 3] = [2, SPAWN_Y + 1, 3];
const LEVER_POS: [i32; 3] = [3, SPAWN_Y + 1, 3];
#[derive(Component)]
struct WanderingChicken;

pub fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline, // Set to offline mode
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(TimePlugin)
        .add_event::<BlockDestroyEvent>()
        .add_event::<CommandResultEvent<BlinkCommand>>()
        .add_command::<BlinkCommand>()
        .add_systems(Startup, setup)
        .add_systems(Update,
                     (init_clients,
                      toggle_gamemode_on_sneak,
                      open_chest,
                      despawn_disconnected_clients,
                      handle_block_destruction,
                      wander_chickens,
                      digging,
                      place_blocks,
                      event_handler,
                      handle_blink_command,
                      handle_blinking
                     ))
        .run();
}

fn handle_blinking(
    mut query: Query<(&mut Blinking, &mut ChunkLayer)>,
    time: Res<Time>,
) {
    for (mut blinking, mut layer) in query.iter_mut() {
        if !blinking.is_active {
            continue;
        }

        // Tick the timer
        blinking.timer.tick(time.delta());

        if blinking.timer.finished() {
            // Extract chunk and block positions
            let (x, y, z) = (REDSTONE_LAMP_POS[0], REDSTONE_LAMP_POS[1], REDSTONE_LAMP_POS[2]);
            let chunk_pos = ChunkPos::new(x >> 4, z >> 4);

            // Access the chunk and toggle the lamp
            if let Some(chunk) = layer.chunk_mut(chunk_pos) {
                // Retrieve the current block state
                let block_state = chunk.block(x as u32, y as u32, z as u32);
                let is_lit = block_state.state.get(PropName::Lit) == Some(PropValue::True);

                // Toggle the `Lit` property
                let new_state = if is_lit {
                    BlockState::REDSTONE_LAMP.set(PropName::Lit, false.into())
                } else {
                    BlockState::REDSTONE_LAMP.set(PropName::Lit, true.into())
                };

                // Set the new block state
                chunk.set_block(x as u32, y as u32, z as u32, new_state);
                layer.set_block([x, y, z], new_state); // This ensures the update propagates correctly

                println!(
                    "Lamp toggled to {}",
                    if is_lit { "OFF" } else { "ON" }
                );
            } else {
                println!("Chunk not loaded for lamp position: {:?}", REDSTONE_LAMP_POS);
            }
        }
    }
}

#[derive(Command, Debug, Clone)]
#[paths("blink")]
enum BlinkCommand {
    #[paths("start")]
    Start,
    #[paths("stop")]
    Stop,
}

fn handle_blink_command(
    mut events: EventReader<CommandResultEvent<BlinkCommand>>,
    mut query: Query<&mut Blinking>,
) {
    for event in events.read() {
        println!("Processing blink command.");
        let mut blinking = query.single_mut();
        match event.result {
            BlinkCommand::Start => {
                blinking.is_active = true;
                blinking.timer.reset(); // Ensure the timer starts fresh
                println!("Blinking started.");
            }
            BlinkCommand::Stop => {
                blinking.is_active = false;
                println!("Blinking stopped.");
            }
        }
    }
}


fn event_handler(
    mut clients: Query<(&Username, &Properties, &UniqueId, &mut Client)>, // Client query
    mut messages: EventReader<ChatMessageEvent>,
    mut block_interacts: EventReader<InteractBlockEvent>,
    mut layers: Query<&mut ChunkLayer>,
) {
    let mut layer = layers.single_mut();

    // Step 1: Extract all messages
    let mut extracted_messages = Vec::new();
    for ChatMessageEvent {
        client,
        message,
        ..
    } in messages.read()
    {
        if let Ok((username, _, _, _)) = clients.get(*client) {
            println!("Message from {}: {:?}", username, message);
            extracted_messages.push((username.clone(), message.clone()));
        }
    }

    // Step 2: Broadcast extracted messages to all clients
    for (username, message) in extracted_messages {
        for (_, _, _, mut client) in &mut clients {
            client.send_chat_message(
                format!("{}: {}", username.as_str(), message).italic(),
            );
        }
    }

    for event in block_interacts.read() {
        if event.position == LEVER_POS.into() {
            // Convert the BlockPos to ChunkPos
            let chunk_pos: ChunkPos = event.position.into();

            // Access the chunk containing the lever
            if let Some(chunk) = layer.chunk_mut(chunk_pos) {
                // Calculate local coordinates within the chunk
                let x = event.position.x as u32;
                let y = event.position.y as u32;
                let z = event.position.z as u32;

                // Get the block state of the lever
                let mut block_state = chunk.block(x, y, z);
                let powered = block_state.state.get(PropName::Powered) == Some(PropValue::True);

                // Toggle the powered state of the lever
                let new_lever_state = block_state.state.set(PropName::Powered, (!powered).into());
                layer.set_block(event.position, new_lever_state); // Update without replacing the lever

                // Update the redstone lamp based on the lever state
                let new_lamp_state = if powered {
                    BlockState::REDSTONE_LAMP.set(PropName::Lit, false.into())
                } else {
                    BlockState::REDSTONE_LAMP.set(PropName::Lit, true.into())
                };
                layer.set_block(REDSTONE_LAMP_POS, new_lamp_state);

                println!(
                    "Lever toggled to {}. Lamp updated to {}.",
                    if !powered { "ON" } else { "OFF" },
                    if !powered { "LIT" } else { "UNLIT" }
                );
            } else {
                println!("Chunk not loaded for the lever position.");
            }
        }
    }



}


#[derive(Debug, Clone, PartialEq, Eq, Event)]
pub struct BlockDestroyEvent {
    pub block_pos: [i32; 3],
    pub client_id: Entity,
}

fn wander_chickens(
    mut chickens: Query<(&mut Position, &mut Velocity), With<WanderingChicken>>,
    server: Res<Server>,
) {
    let mut rng = rand::thread_rng();
    let delta_time = 1.0 / server.tick_rate().get() as f32;

    for (mut pos, mut vel) in chickens.iter_mut() {
        if rng.gen_bool(0.1) { // 10% chance to change direction
            let angle = rng.gen_range(0.0..std::f64::consts::TAU) as f32;
            vel.0 = Vec3::new(angle.cos(), 0.0, angle.sin());
        }
        pos.0.x += vel.0.x as f64 * delta_time as f64;
        pos.0.z += vel.0.z as f64 * delta_time as f64;
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -50..50 {
        for x in -50..50 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }


    println!("Initializing server...");
    // Load the level file
    let level_data = fs::read_to_string("FMIS01.MIE").expect("Failed to read level file");

    // Parse the map manually
    let map_start = level_data.find("map = {").expect("Map section not found");
    let map_end = level_data.find("}").expect("End of map section not found");
    let map_data = &level_data[map_start + 7..map_end];

    let level_map: Vec<String> = map_data
        .lines()
        .map(|line| line.trim()) // Trim leading/trailing whitespace
        .filter(|line| !line.is_empty() && !line.starts_with("map") && !line.starts_with('{') && !line.starts_with('}'))
        .map(|line| line.trim_matches(&['"', ',', ' '][..]).to_string()) // Remove quotes, commas, and extra spaces
        .collect();

    let map_height = level_map.len();

    for (y, row) in level_map.iter().enumerate() {
        let reversed_y = map_height - 1 - y; // Reverse Y-axis
        for (x, tile) in row.chars().enumerate() {
            print!("{}", tile);
            let block_state = match tile {
                '.' => continue,
                'F' => BlockState::STONE,
                'I' => BlockState::GLASS,
                'H' => BlockState::BIRCH_WOOD,
                '6' => BlockState::COBBLESTONE,
                _ => BlockState::DIRT,
            };

            // Create depth by duplicating tiles along the Z-axis
            for z_offset in 0..3 {
                layer.chunk.set_block(
                    [x as i32, SPAWN_Y + reversed_y as i32, z_offset as i32],
                    block_state,
                );
            }

        }
        println!(".");
    }

    // Add blocks that we're watching
    layer.chunk.set_block(CHEST_POS, BlockState::CHEST);
    layer.chunk.set_block(REDSTONE_LAMP_POS, BlockState::REDSTONE_LAMP);
    layer.chunk.set_block(LEVER_POS, BlockState::LEVER.set(PropName::Facing, PropValue::East),);
    // Clone the layer to reuse it

    let layer_id = commands.spawn((
        layer, // This includes the ChunkLayer component
        Blinking {
            is_active: false,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating), // 1-second interval
        },
    )).id();

     // = commands.spawn(layer_clone).id();

    // commands.spawn(layer);

    let inventory = Inventory::with_title(
        InventoryKind::Generic9x3,
        "Extra".italic() + " ChestyX".not_italic().bold().color(Color::BLUE) + " Chest".not_italic(),
    );

    commands.spawn(inventory);

    // Spawn a chicken
    // commands.spawn((
    //     ChickenEntityBundle {
    //         layer: EntityLayerId(layer_id),
    //         position: Position::new([1.0, f64::from(SPAWN_Y) + 1.0, 0.0]),
    //         ..Default::default()
    //     },
    //     WanderingChicken,
    // ));


    // commands.spawn(Inventory::with_title(
    //     InventoryKind::Generic9x3,
    //     "Destructible Blocks".italic().bold().color(Color::GREEN),
    // ));

}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut Look,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut look,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([8.0, 65.0, 20.0]);

        // Adjust the yaw to turn the avatar around 180 degrees
        look.yaw = 195.0; // Yaw in degrees, 180 = facing the opposite direction
        look.pitch = -20.0; // Pitch should remain unchanged unless you want to look up/down

        *game_mode = GameMode::Creative;
        client.send_chat_message(
            "Welcome to Digital Twin world.".italic(),
        );
    }
}


fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut GameMode>,
    mut events: EventReader<SneakEvent>,
) {
    for event in events.read() {
        if let Ok(mut mode) = clients.get_mut(event.client) {
            if event.state == SneakState::Start {
                *mode = match *mode {
                    GameMode::Survival => GameMode::Creative,
                    GameMode::Creative => GameMode::Survival,
                    _ => GameMode::Creative,
                };
            }
        }
    }
}


fn handle_block_destruction(
    mut events: EventReader<BlockDestroyEvent>,
    mut layers: Query<&mut ChunkLayer>,
) {
    for event in events.read() {
        if event.block_pos == LEVER_POS {
            // Prevent the lever from being destroyed
            println!("Attempt to destroy lever blocked.");
            continue;
        }

        if let Ok(mut layer) = layers.get_single_mut() {
            // Convert block position to ChunkPos via BlockPos.
            let block_pos = BlockPos::new(event.block_pos[0], event.block_pos[1], event.block_pos[2]);
            let chunk_pos: ChunkPos = block_pos.into();

            // Get a mutable reference to the chunk.
            if let Some(chunk) = layer.chunk_mut(chunk_pos) {
                chunk.set_block(
                    event.block_pos[0] as u32,
                    event.block_pos[1] as u32,
                    event.block_pos[2] as u32,
                    BlockState::AIR,
                );
                println!(
                    "Block at {:?} destroyed by client {:?}",
                    event.block_pos, event.client_id
                );
            } else {
                println!("Chunk not found for block position {:?}", event.block_pos);
            }
        } else {
            println!("No layer found to handle block destruction.");
        }
    }
}

fn open_chest(
    mut commands: Commands,
    inventories: Query<Entity, (With<Inventory>, Without<Client>)>,
    mut events: EventReader<InteractBlockEvent>,
) {
    for event in events.read() {
        if event.position != CHEST_POS.into() {
            continue;
        }
        let open_inventory = OpenInventory::new(inventories.single());
        commands.entity(event.client).insert(open_inventory);
    }
}



fn digging(
    clients: Query<&GameMode>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        if event.position == LEVER_POS.into() {
            // Prevent the lever from being replaced or removed
            println!("Digging event ignored for lever.");
            continue;
        }

        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };

        if (*game_mode == GameMode::Creative && event.state == DiggingState::Start)
            || (*game_mode == GameMode::Survival && event.state == DiggingState::Stop)
        {
            layer.set_block(event.position, BlockState::AIR);
        }
    }
}

fn place_blocks(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((mut inventory, game_mode, held)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            // can't place this item as a block
            continue;
        };

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count > 1 {
                let amount = stack.count - 1;
                inventory.set_slot_amount(slot_id, amount);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }
        let real_pos = event.position.get_in_direction(event.face);
        let state = block_kind.to_state().set(
            PropName::Axis,
            match event.face {
                Direction::Down | Direction::Up => PropValue::Y,
                Direction::North | Direction::South => PropValue::Z,
                Direction::West | Direction::East => PropValue::X,
            },
        );
        layer.set_block(real_pos, state);
    }
}
