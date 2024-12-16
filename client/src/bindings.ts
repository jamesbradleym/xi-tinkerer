// @ts-nocheck
         // This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

         /** user-defined commands **/

         export const commands = {
async dummyEventTypeGen() : Promise<Result<[FileNotification, DatProcessorMessage], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("dummy_event_type_gen") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async selectFfxiFolder(path: string | null) : Promise<Result<string | null, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("select_ffxi_folder", { path }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async selectProjectFolder(path: string | null) : Promise<Result<string[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("select_project_folder", { path }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async loadPersistenceData() : Promise<Result<PersistenceData, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("load_persistence_data") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getMiscDats() : Promise<Result<DatDescriptor[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_misc_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getStandaloneStringDats() : Promise<Result<DatDescriptor[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_standalone_string_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getItemDats() : Promise<Result<DatDescriptor[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_item_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getGlobalDialogDats() : Promise<Result<DatDescriptor[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_global_dialog_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async browseDats() : Promise<Result<BrowseInfo[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("browse_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getZonesForType(datDescriptor: DatDescriptor) : Promise<Result<ZoneInfo[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_zones_for_type", { datDescriptor }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getWorkingFiles() : Promise<Result<DatDescriptor[], any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("get_working_files") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async makeAllDats() : Promise<Result<null, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("make_all_dats") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async makeDat(datDescriptor: DatDescriptor) : Promise<Result<null, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("make_dat", { datDescriptor }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async makeYaml(datDescriptor: DatDescriptor) : Promise<Result<null, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("make_yaml", { datDescriptor }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async copyLookupTables() : Promise<Result<null, any>> {
try {
    return { status: "ok", data: await TAURI_INVOKE("copy_lookup_tables") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
}
}

         /** user-defined events **/



         /** user-defined statics **/

         

/** user-defined types **/

export type BrowseInfo = { path: string; id: number }
export type DatDescriptor = { type: "DataMenu" } | { type: "AbilityNames" } | { type: "AbilityDescriptions" } | { type: "AreaNames" } | { type: "AreaNamesAlt" } | { type: "CharacterSelect" } | { type: "ChatFilterTypes" } | { type: "DayNames" } | { type: "Directions" } | { type: "EquipmentLocations" } | { type: "ErrorMessages" } | { type: "IngameMessages1" } | { type: "IngameMessages2" } | { type: "JobNames" } | { type: "KeyItems" } | { type: "MenuItemsDescription" } | { type: "MenuItemsText" } | { type: "MoonPhases" } | { type: "PolMessages" } | { type: "RaceNames" } | { type: "RegionNames" } | { type: "SpellNames" } | { type: "SpellDescriptions" } | { type: "StatusInfo" } | { type: "StatusNames" } | { type: "TimeAndPronouns" } | { type: "Titles" } | { type: "Misc1" } | { type: "Misc2" } | { type: "WeatherTypes" } | { type: "Armor" } | { type: "Armor2" } | { type: "Currency" } | { type: "GeneralItems" } | { type: "GeneralItems2" } | { type: "PuppetItems" } | { type: "UsableItems" } | { type: "Weapons" } | { type: "VouchersAndSlips" } | { type: "Monipulator" } | { type: "Instincts" } | { type: "MonsterSkillNames" } | { type: "StatusNamesDialog" } | { type: "EmoteMessages" } | { type: "SystemMessages1" } | { type: "SystemMessages2" } | { type: "SystemMessages3" } | { type: "SystemMessages4" } | { type: "UnityDialogs" } | { type: "EntityNames"; index: number } | { type: "Dialog"; index: number } | { type: "Dialog2"; index: number } | { type: "Event"; index: number }
export type DatProcessingState = "Working" | { Finished: string } | { Error: string }
export type DatProcessorMessage = { dat_descriptor: DatDescriptor; output_kind: DatProcessorOutputKind; state: DatProcessingState }
export type DatProcessorOutputKind = "Dat" | "Yaml"
export type FileNotification = { dat_descriptor: DatDescriptor; is_delete: boolean }
export type PersistenceData = { ffxi_path: string | null; recent_projects: string[] }
export type ZoneInfo = { id: number; name: string }

/** tauri-specta globals **/

         import { invoke as TAURI_INVOKE } from "@tauri-apps/api/core";
import * as TAURI_API_EVENT from "@tauri-apps/api/event";
import { type WebviewWindow as __WebviewWindow__ } from "@tauri-apps/api/webviewWindow";

type __EventObj__<T> = {
  listen: (
    cb: TAURI_API_EVENT.EventCallback<T>
  ) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
  once: (
    cb: TAURI_API_EVENT.EventCallback<T>
  ) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
  emit: T extends null
    ? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
    : (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> =
  | { status: "ok"; data: T }
  | { status: "error"; error: E };

function __makeEvents__<T extends Record<string, any>>(
  mappings: Record<keyof T, string>
) {
  return new Proxy(
    {} as unknown as {
      [K in keyof T]: __EventObj__<T[K]> & {
        (handle: __WebviewWindow__): __EventObj__<T[K]>;
      };
    },
    {
      get: (_, event) => {
        const name = mappings[event as keyof T];

        return new Proxy((() => {}) as any, {
          apply: (_, __, [window]: [__WebviewWindow__]) => ({
            listen: (arg: any) => window.listen(name, arg),
            once: (arg: any) => window.once(name, arg),
            emit: (arg: any) => window.emit(name, arg),
          }),
          get: (_, command: keyof __EventObj__<any>) => {
            switch (command) {
              case "listen":
                return (arg: any) => TAURI_API_EVENT.listen(name, arg);
              case "once":
                return (arg: any) => TAURI_API_EVENT.once(name, arg);
              case "emit":
                return (arg: any) => TAURI_API_EVENT.emit(name, arg);
            }
          },
        });
      },
    }
  );
}

     