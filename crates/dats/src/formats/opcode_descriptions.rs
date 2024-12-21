use super::event::OpcodeSizeCallback;

pub const DESCRIPTIONS: [(u8, &str, &'static [usize], Option<OpcodeSizeCallback>); 218] =
[
    (0x00, "Ends the current ReqStack execution; resetting it back to defaults.", &[1], None),
    (0x01, "Directly sets the ExecPointer position.", &[3], None),
    (0x02, "Handles multiple types of if conditional statements.", &[8], None),
    (0x03, "Gets a value then stores it.", &[5], None),
    (0x04, "Deprecated. This opcode appears to be deprecated, it does nothing.", &[3], None),
    (0x05, "Sets a value to 1.", &[3], None),
    (0x06, "Sets a value to 0.", &[3], None),
    (0x07, "Adds two values then stores the result.", &[5], None),
    (0x08, "Subtracts two values then stores the result.", &[5], None),
    (0x09, "Sets a bit flag value then stores the result.", &[5], None),
    (0x0A, "Clears a bit flag value then stores the result", &[5], None),
    (0x0B, "Increments a value then store it.", &[3], None),
    (0x0C, "Decrements a value then store it.", &[3], None),
    (0x0D, "Gets the bitwise AND result of two values and stores it.", &[5], None),
    (0x0E, "Gets the bitwise OR result of two values and stores it.", &[5], None),
    (0x0F, "Gets the bitwise XOR result of two values and stores it.", &[5], None),
    (0x10, "Gets the bitwise left-shift result of two values and stores it.", &[5], None),
    (0x11, "Gets the bitwise right-shift result of two values and stores it.", &[5], None),
    (0x12, "Generates a random number via rand() and stores it.", &[3], None),
    (0x13, "Generates a random number via rand(, &[1], None), with a given remainder, and stores it.", &[5], None),
    (0x14, "Gets the product of two values and stores it.", &[5], None),
    (0x15, "Gets the quotient of two values and stores it.", &[5], None),
    (0x16, "Performs a sin operation on two values and stores the result.", &[7], None),
    (0x17, "Performs a cos operation on two values and stores the result.", &[7], None),
    (0x18, "Performs an atan2 operation on two values and stores the result.", &[7], None),
    (0x19, "Reads two values and stores them in flipped order. (Endian swap.)", &[5], None),
    (0x1A, "Jumps to a new position in the event data.", &[3], None),
    (0x1B, "Returns from the most recent jump on the JumpStack.", &[1], None),
    (0x1C, "Sets, or updates (decreases, &[1], None), the current ReqStack[RunPos].WaitTime value.", &[3], None),
    (0x1D, "Loads and prints an event message to chat, using EntityTargetIndex[1] as the speaker.", &[3], None),
    (0x1E, "Tells an entity to look at another entity and begin 'talking'. (This puts the 'talking' entity into an animation where their mouth moves.)", &[5], None),
    (0x1F, "Updates the event position information.", &[2, 8], None),//Some(determine_opcode_0x1f_size)),
    (0x20, "Sets the CliEventUcFlag flag value. (This flag is used to lock the player from controlling their character.)", &[2], None),
    (0x21, "Sets the EventExecEnd flag value to 1.", &[1], None),
    (0x22, "Calls XiAtelBuff::SetEventHideFlag for the current event entity.", &[2], None),
    (0x23, "Waits for the local player to interact with a dialog message.", &[1], None),
    (0x24, "Creates a dialog window with selectable options for the player to choose from.", &[7], None),
    (0x25, "Waits for a dialog select (created by opcode 0x0024) to be made by the player.", &[1], None),
    (0x26, "Yields the event VM. Note: This opcode may be deprecated. Since it only ever sets the RetFlag the opcode will never advance further leaving it in an endless self-handled cycle each time the VM is ticked.", &[1], None),
    (0x27, "Calls a helper FUNC_REQSet which in turn calls XiEvent::ReqSet after checking some conditions.", &[7], None),
    (0x28, "Similar to opcode 0x0027, but with extra checks/conditions. The function starts by checking for the current ReqStack[RunPos].ReqFlag being set, then will do a similar check setup to FUNC_REQSet but will end with calling XiEvent::GetReqStatus instead.", &[7], None),
    (0x29, "Similar to opcode 0x0028.", &[7], None),
    (0x2A, "Similar to opcode 0x0028.", &[6], None),
    (0x2B, "Loads and prints an event message with the given entity as the speaker. This handler works similar to 0x001D, however, the opcode holds the entity information used as the speaker.", &[7], None),
    (0x2C, "Creates and loads a CMoSchedularTask on the desired entity. (Appears to set an entity action.)", &[13], None),
    (0x2D, "Creates and loads a zone based CMoSchedularTask on the desired entities. (Appears to schedule a zone action.)", &[13], None),
    (0x2E, "Sets the CliEventCancelSetData flag. If CliEventCancelSetFlag is set, also sets the CliEventCancelFlag flag.", &[1], None),
    (0x2F, "Adjusts the given entities Render.Flag0 value.", &[6], None),
    (0x30, "Sets the ucoff_continue flag to 0.", &[1], None),
    (0x31, "Updates the event position information.", &[2, 10], None),
    (0x32, "Sets the ExtData[1]->MainSpeed value.", &[3], None),
    (0x33, "Adjusts the event entities Render.Flags0 value.", &[2], None),
    (0x34, "Appears to load and unload an additional zone to be used with the event.", &[3], None),
    (0x35, "Similar to opcode 0x0034. This appears to load an additional zone for the event, however this handler does not have a call to XiZone::Close.", &[3], None),
    (0x36, "Updates the current ExtData[1]->EventPos information, calibrates the current event entity position then calls XiAtelBuff::CopyAllPosEvent and XiAtelBuff::ReqExecHitCheck.", &[7], None),
    (0x37, "Updates the current ExtData[1]->EventPos and ExtData[1]->EventDir[1] information, calibrates the current event entity position then calls XiAtelBuff::CopyAllPosEvent and XiAtelBuff::ReqExecHitCheck.", &[9], None),
    (0x38, "Sets the lower-word of CliEventModeLocal to a masked value. CliEventModeLocal is used to tell the client how the event should alter the client state. ", &[3], None),
    (0x39, "Sets the current ExtData[1]->EventDir[1] value.", &[3], None),
    (0x3A, "Converts a float Yaw value to it's single byte representation and stores it.", &[7], None),
    (0x3B, "Gets the current position of the given entity (or uses the ExtData[1]->EventPos depending on flags) and stores it.", &[11], None),
    (0x3C, "Compares two values (using a shift). If condition is met, sets a bit flag and stores the result.", &[7], None),
    (0x3D, "Compares two values (using a shift). If condition is met, clears a bit flag and stores the result.", &[7], None),
    (0x3E, "Tests if a bit is set. Adjusts the ExecPointer based on the state of the flag.", &[7], None),
    (0x3F, "Calculates the remainder of two values and stores the result.", &[7], None),
    (0x40, "Sets a bit flag value and stores it. One usage of this opcode is to tell the client which dialog menu options are enabled/available.", &[9], None),
    (0x41, "Gets a bit flag value and stores it. One usage of this opcode is to tell the client which dialog menu options are enabled/available.", &[9], None),
    (0x42, "Sets the CliEventCancelSetData flag to 0. If CliEventCancelSetFlag is set, then CliEventCancelFlag is also set to 0.", &[1], None),
    (0x43, "Used to tell the server the server when the client has updated an event or has completed it.", &[2], None),
    (0x44, "Tests if the given entity is valid. Adjusts the ExecPointer based on the result.", &[5], None),
    (0x45, "Loads and starts a scheduled task with the given two entities.", &[17], None),
    (0x46, "Enables and disables the player camera control. Also disables rendering some menus to allow the game to play cutscenes without unneeded info on screen.", &[2, 4], None),//Some(determine_opcode_0x46_size)),
    (0x47, "Updates the players location during an event. This opcode will send an 0x005C packet to the server to inform it of your position change.", &[2, 10], None),//Some(determine_opcode_0x47_size)),
    (0x48, "Loads and prints an event message to chat, without a speaker entity.", &[3], None),
    (0x49, "Loads and prints an event message to chat, without a speaker entity.", &[7], None),
    (0x4A, "Tells an entity to look at another entity.", &[9], None),
    (0x4B, "Updates the given entities yaw direction.", &[7], None),
    (0x4C, "Sets the event entities StatusEvent to 8 if a specific Render.Flags0 bit is not set. (Open door.)", &[1], None),
    (0x4D, "Sets the event entities StatusEvent to 9 if a specific Render.Flags0 bit is not set. (Close door.)", &[1], None),
    (0x4E, "Sets the entities event hide flag within Render.Flags0.", &[6], None),
    (0x4F, "Sets the event entities StatusEvent to the given value if a specific Render.Flags0 bit is not set.", &[3], None),
    (0x50, "Ends a CMoSchedularTask.", &[13], None),
    (0x51, "Ends a zone based CMoSchedularTask.", &[13], None),
    (0x52, "Ends a CMoSchedularTask. (Load / Main)", &[15], None),
    (0x53, "Waits for the given entities schedular to finish its current action.", &[13], None),
    (0x54, "Waits for the zone schedular to finish its current action.", &[13], None),
    (0x55, "Waits for the Main/Load schedular to finish its current action.", &[15], None),
    (0x56, "Deprecated. This opcode does not do anything with the values it reads anymore. This appears to be deprecated.", &[5], None),
    (0x57, "Creates a frame delay from the current frame delay value and stores it.", &[3], None),
    (0x58, "Yields the event VM.", &[3], None),
    (0x59, "Handles multiple cases regarding updating an entities data for events.", &[4, 6, 7, 8], None),
    (0x5A, "Updates the event position information.", &[2, 8], None),
    (0x5B, "Loads an extended schedular task.", &[15, 17], None),
    (0x5C, "Handles multiple cases regarding the music player.", &[4, 6], None),
    (0x5D, "Sets, or eases, the current playing music to a new volume.", &[5], None),
    (0x5E, "Appears to stop the event entities current action and reset them back to an idle motion.", &[5], None),
    (0x5F, "This handler has a few cases, most of which call other opcode handlers and react based on their returns.", &[2, 7, 14, 16, 18], None),
    (0x60, "Handler with multiple use cases. The default case where the opcode was two bytes long was deprecated and just skipped now. Adjusts the event entities Render.Flags1 value.", &[2, 4, 6], None),
    (0x61, "Adjusts the event entities Render.Flags2 value.", &[2], None),
    (0x62, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0x63, "Sets the event entity to play an animation then waits for it to complete.", &[3], None),
    (0x64, "Calculates and stores the distance between the given points.", &[11], None),
    (0x65, "Calculates and stores the 3D distance between the given entities.", &[11], None),
    (0x66, "Handler that calls the same helper call as opcode 0x005B, just with a different arguments.", &[15, 17], None),
    (0x67, "Tells the client to hide the entire HUD UI elements during the cutscene. (ie. The compass, status icons, chat, menus, etc.)", &[5], None),
    (0x68, "Tells the client to unhide the entire HUD UI elements. (ie. The compass, status icons, chat, menus, etc.)", &[1], None),
    (0x69, "Sets the sound volume of the desired sound type.", &[4], None),
    (0x6A, "Changes the sound volume of the desired sound type.", &[4], None),
    (0x6B, "Appears to stop the given entities current action and reset them back to an idle motion.", &[9], None),
    (0x6C, "Fades an enities color in and out. This can be used to both set just the alpha of the entity, but also the color. This works in stages to allow the color to fade in and/or out smoothly, or immediately, depending on the time values set.", &[9], None),
    (0x6D, "Deprecated. This opcode appears to be deprecated, it does nothing.", &[7], None),
    (0x6E, "Sets the given entity to play an emote animation.", &[7], None),
    (0x6F, "Delays the event VM execution until ReqStack[RunPos].WaitTime has reached 0. Used as a yieldable sleep call.", &[1], None),
    (0x70, "Checks the event entity for a render flag, yields if set. Otherwise, cancels the entity movement and advances.", &[1], None),
    (0x71, "Handles the usage of string input from the player during events. Such as password prompts and similar.", &[2, 4, 6, 8, 10], None),
    (0x72, "Appears to load event based weather information and update the weather accordingly for it.", &[4, 6, 10], None),
    (0x73, "Schedules tasks for casting magic on the two given entities.", &[11], None),
    (0x74, "Adjusts the event entities Render.Flags1 value.", &[2], None),
    (0x75, "Loads a room and updates the players sub-region with the server.", &[4, 6, 8], None),
    (0x76, "Checks the given entities Render.Flags0 and Render.Flags3 and yields if successful.", &[5], None),
    (0x77, "Disables the game clock and sets the client to a specific time for the event. Can also set the weather at the same time.", &[5], None),
    (0x78, "Enables the game timer and resets the zone weather.", &[1], None),
    (0x79, "Used to look at / rotate towards another entity.", &[10, 12], None),//Some(determine_opcode_0x0079_size)),
    (0x7A, "Handles multiple entity conditions dependant on following event byte cases.", &[2, 6, 7, 8], None),
    (0x7B, "Unsets the given entities talking status, setting their NpcSpeechFrame back to -1.", &[5], None),
    (0x7C, "Adjusts the given entities Render.Flags2 value.", &[5], None),
    (0x7D, "Loads and starts a scheduled task using the local player as the entity. (Appears to be used to display rank up animations.)", &[3], None),
    (0x7E, "Multi-purpose opcode relating to chocobos and mounts.", &[6, 8, 16, 18], None),
    (0x7F, "Waits for a dialog select to be made by the player.", &[1], None),
    (0x80, "Tests the given entity for several conditions. Yields or moves forward depending on the results. (Appears to be used to check if the entity is loading an action or similar.)", &[5], None),
    (0x81, "Sets an unknown value in the given entities warp data.", &[6], None),
    (0x82, "Finds and hit tests a rect based on the current event entities position.", &[7], None),
    (0x83, "Gets and stores the current game time.", &[3], None),
    (0x84, "Adjusts the event entities Render.Flags3 value.", &[1], None),
    (0x85, "Opens a mog house sub-menu depending on the passed parameter.", &[1], None),
    (0x86, "Adjusts the given entities Render.Flags3 value.", &[6], None),
    (0x87, "Used for handling the generation of world passes. Sends 0x001B packets to handle the various world pass functionalities.", &[2], None),
    (0x88, "Used for handling the generation of world passes. Sends 0x001B packets to handle the various world pass functionalities.", &[2], None),
    (0x89, "Opens the desired map (ie. /map, &[1], None), preparing it for usage within the event. (ie. NPCs that mark your map/show you around.)", &[3], None),
    (0x8A, "Closes the map window. (ie. after being opened via opcode 0x0089)", &[1], None),
    (0x8B, "Sets, or updates, a marker point on the players map. (ie. Used by NPCs that help new players and mark your map.)", &[25], None),
    (0x8C, "This handler is used for multiple purposes, related to crafting. (ie. Requesting recipes, synth support, and similar.)", &[2, 8, 10, 12, 14], None),
    (0x8D, "Opens the map window with the given properties. This handler is used mainly when an NPC opens your map but it is not with the sub-menus visible. Mainly to show an overview of the map with no extra bloat on screen or markings on the map.", &[5], None),
    (0x8E, "Sets the event entities event status to 45 if valid.", &[1], None),
    (0x8F, "Sets the event entities event status to 46 if valid.", &[1], None),
    (0x90, "Adjusts the event entities Render.Flags0 and Render.Flags1 values.", &[1], None),
    (0x91, "Sets the ExtData[1].MainSpeedBase value.", &[3], None),
    (0x92, "Adjusts the given entities Render.Flags3 value.", &[6], None),
    (0x93, "Appears to display an items information. (Perhaps the same manner with how crafting shows results?)", &[3], None),
    (0x94, "Adjusts the given entities Render.Flags3 value.", &[6], None),
    (0x95, "Sets the event entity up for being an event based npc. Cleans up the event entities attachments.", &[3], None),
    (0x96, "Unsets the event entity from being an event based npc.", &[1], None),
    (0x97, "Saves the current zone WindBase and WindWidth values then sets new ones.", &[5], None),
    (0x98, "Yields if the zone is loading data, continues otherwise.", &[1], None),
    (0x99, "Yields if the given entity is playing an animation, continues otherwise.", &[5], None),
    (0x9A, "Yields until the music server is no longer reading data.", &[1], None),
    (0x9B, "Yields if the event entity is playing an animation, continues otherwise.", &[1], None),
    (0x9C, "Stores the client language id.", &[3], None),
    (0x9D, "Handler that has multiple purposes, mainly focused around handling strings.", &[6, 8, 9, 10, 23], None),
    (0x9E, "Sets the PTR_RectEventSendFlag value.", &[2], None),
    (0x9F, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xA0, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xA1, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xA2, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xA3, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xA4, "Adjusts the event entities Render.Flags3 value.", &[2], None),
    (0xA5, "Adjusts the event entities Render.Flags3 value.", &[2], None),
    (0xA6, "Requests the event map number from the server by sending a 0x00EB packet. Sets the PTR_RecvEventMapNumFlag to mark the client as awaiting for a response and then yields until it is unset.", &[1, 4], None),
    (0xA7, "Waits for the server to respond to a client request. This is used with battlefield registration NPCs. (ie. Dynamis, Moblin Maze Mongers, Salvage, etc.)", &[12, 4], None),
    (0xA8, "Opens the map (if requested, &[1], None), unlocks and renames markers.", &[6], None),
    (0xA9, "Disables the game time and sets it to a specific given time.", &[3], None),
    (0xAA, "Gets a value to be used as a Vana'diel timestamp. Converts that timestamp into the various time parts and stores them.", &[17], None),
    (0xAB, "Handles various sub-cases; mostly dealing with altering entity render flags.", &[2, 4, 6], None),
    (0xAC, "Handles multiple sub-cases.", &[4, 6, 8], None),
    (0xAD, "Handler with multiple sub-cases, used to do various scheduler actions against the two given entities.", &[12], None),
    (0xAE, "Handles multiple sub-cases. Doesn't seem to have any specific purpose.", &[6, 8, 10], None),
    (0xAF, "Gets and stores the camera position values.", &[8], None),
    (0xB0, "Loads and prints an event message to chat. Uses the given entities as the speaker and listener.", &[12], None),
    (0xB1, "Gets and stores the value of a flag. PTR_UnknownValue is part of the main app object which is initialized to 128. This valid doesn't seem to ever change, and has been the same since the original beta of the game. At this time, the purpose of this value is unknown.", &[4], None),
    (0xB2, "Handler has two modes. The first mode requests opening the delivery box. The second mode is to wait a certain amount of time, used to wait for the delivery box to open.", &[2, 4], None),
    (0xB3, "This handler is used for dealing with the rankings boards. For example, the fishing rank boards with Chenon in Selbina.", &[2, 4, 14, 18], None),
    (0xB4, "Handler with multiple sub-usages.", &[2, 3, 4, 6, 12, 20], None),
    (0xB5, "Sets the current event entities name.", &[4], None),
    (0xB6, "Handler with multiple sub-usages. Related to entity looks / gear visuals.", &[2, 4, 6, 14, 16, 20], None),
    (0xB7, "Handler with multiple sub-usages.", &[8, 10], None),
    (0xB8, "Opens the map (if requested, &[1], None), adds and sets markers.", &[27], None),
    (0xB9, "Opens the map (if requested, &[1], None), edits and renames a marker. (Name is taken from the event Read buffer.)", &[8], None),
    (0xBA, "Obtains the given entity, if valid, attempts to calibrate its position then calls XiAtelBuff::CopyAllPosEvent and XiAtelBuff::ReqExecHitCheck.", &[13], None),
    (0xBB, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xBC, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xBD, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xBE, "Stores the current ReqStack[RunPos].WhoServerId value.", &[3], None),
    (0xBF, "Handler that is used for chocobo racing. This handler has debug messages left in, so it can be translated to actual opcode names.", &[8, 10], None),
    (0xC0, "Adjusts the event entities Render.Flags3 value.", &[3], None),
    (0xC1, "Obtains the given entity, tests it for something. If successful, then the last action is killed and its resp data is deleted.", &[5], None),
    (0xC2, "The purpose of this opcode is currently unknown. This makes use of the internal party state object, checking for flags/values. These check if a flag is set that is more recently added to the party structure.", &[2, 4, 6], None),
    (0xC3, "Copies a string value into an unknown buffer array.", &[7], None),
    (0xC4, "Handler that calls the same helper call as opcode 0x0073, just with a different arguments.", &[11], None),
    (0xC5, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xC6, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xC7, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xC8, "Opens the map window with the given parameters.", &[7], None),
    (0xC9, "Enables the game timer.", &[1], None),
    (0xCA, "Deprecated. No handler exists for this opcode at this time.", &[1], None),
    (0xCB, "Deprecated. No handler exists for this opcode at this time.", &[1], None),
    (0xCC, "This opcode appears to be used to open and display information windows for various things. Mainly items.", &[4, 6, 10, 14], None),
    (0xCD, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xCE, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xCF, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xD0, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xD1, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xD2, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xD3, "Gets the given entity and calls a helper function that clears its motion queue lists.", &[6], None),
    (0xD4, "Handles multiple sub-opcodes. These appear to be related to opening the map and querying the user for input.", &[2, 6, 8, 12], None),
    (0xD5, "Handler that calls the same helper call as opcode 0x0045, just with a different second argument.", &[17], None),
    (0xD6, "Handler that calls the same helper call as opcode 0x0055, just with a different second argument.", &[15], None),
    (0xD7, "Handler that calls the same helper call as opcode 0x0052, just with a different second argument.", &[15], None),
    (0xD8, "Sets the ExtData[1]->EventDir information for the given entity.", &[6, 8, 12], None),
    (0xD9, "Sets an unknown flag value.", &[2], None),
];

// fn determine_opcode_0x1f_size(opcode: u8, data: &[u8], previous_opcodes: &[EventOpcode]) -> Option<usize> {
//     if opcode != 0x1F || data.len() < 1 {
//         return None; // Invalid or insufficient data
//     }

//     match data[0] {
//         0x00 => Some(8), // Initialization mode
//         0x01 => Some(2), // Update mode
//         _ => None,       // Invalid or unknown mode
//     }
// }

// fn determine_opcode_0x46_size(opcode: u8, data: &[u8], previous_opcodes: &[EventOpcode]) -> Option<usize> {
//     // Enables and disables the player camera control.
//     // Also disables rendering some menus to allow the game to play cutscenes without unneeded info on screen.

//     // Based on event bytes within an event series it seems this is called in pairs to start and end camera for cutscenes
//     if opcode != 0x46 {
//         return None; // Invalid opcode
//     }

//     // Count how many times 0x46 has been called in the current series
//     let call_count = previous_opcodes.iter().filter(|op| op.opcode == 0x46).count();

//     if call_count % 2 == 0 {
//         Some(2) // First call or every second call uses 4 bytes
//     } else {
//         Some(4) // Odd calls after the first use 2 bytes
//     }
// }

// fn determine_opcode_0x47_size(opcode: u8, data: &[u8], previous_opcodes: &[EventOpcode]) -> Option<usize> {
//     if opcode != 0x47 {
//         return None; // Invalid opcode
//     }

//     // Count how many times 0x47 has been called in the current series
//     let call_count = previous_opcodes.iter().filter(|op| op.opcode == 0x47).count();

//     if call_count % 2 == 0 {
//         Some(10) // First call or every second call uses 10 bytes
//     } else {
//         Some(2) // Odd calls after the first use 2 bytes
//     }
// }

// fn determine_opcode_0x0079_size(opcode: u8, data: &[u8], previous_opcodes: &[EventOpcode]) -> Option<usize> {
//     if opcode != 0x79 || data.len() < 2 {
//         return None; // Invalid opcode or insufficient data
//     }

//     match data[0] { // Equivalent to this->EventData[this->ExecPointer + 1]
//         1 => Some(12), // 12-byte size
//         2 => Some(10), // 10-byte size
//         0 => Some(10), // Default case
//         _ => None,     // Unknown or invalid
//     }
// }