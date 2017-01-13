import {Database} from "./runtime/runtime";

export enum Owner {client, server};
export enum Mode {workspace, file};

export interface Config {
  path?:string,
  runtimeOwner?: Owner,
  controlOwner?: Owner,
  port?:number,
  editor?: boolean,
  root?: string,
  eveRoot?: string,
  internal?: boolean,
  mode?: Mode,
  initJsonDB? : string[],
  databases? : {[name:string]:Database}
}

export var config:Config = {};

export function init(opts:Config) {
  for(let key in opts) {
    config[key] = opts[key];
  }
}
