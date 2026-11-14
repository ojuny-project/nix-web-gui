use std::collections::BTreeMap;

use nix_web_gui_engine::api::{menu_json, page_json, record_json};
use nix_web_gui_engine::apply;
use nix_web_gui_engine::authoring;
use nix_web_gui_engine::catalog;
use nix_web_gui_engine::config::WebConfig;
use nix_web_gui_engine::flake;
use nix_web_gui_engine::git;
use nix_web_gui_engine::preview;
use nix_web_gui_engine::render;
use nix_web_gui_engine::secureboot;
use nix_web_gui_engine::session::SettingsSession;
use nix_web_gui_engine::users;
use serde_json::{json, Value};

pub fn handle(session: &mut Option<SettingsSession>, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    let result: Result<Value, (i32, String)> = match method {
        "echo" => Ok(params),
        "session.open" => open_settings(session, &params),
        "session.close" => {
            *session = None;
            Ok(json!({ "closed": true }))
        }
        "session.reload" => with_settings(session, |settings| {
            settings.reload().map_err(|error| (-32013, error))?;
            Ok(json!({
                "root": settings.root,
                "menus": settings.menus().len(),
                "records": settings.records().count(),
                "staged": settings.staged().len(),
            }))
        }),
        "session.discard" => with_settings(session, |settings| {
            settings.discard().map_err(|error| (-32013, error))?;
            Ok(json!({
                "root": settings.root,
                "menus": settings.menus().len(),
                "records": settings.records().count(),
                "staged": 0,
            }))
        }),
        "session.state" => with_settings(session, |settings| {
            Ok(json!({
                "root": settings.root,
                "menus": settings.menus().len(),
                "records": settings.records().count(),
                "staged": settings.staged().len(),
            }))
        }),
        "scan" => with_settings(session, |settings| {
            let menus = settings
                .menus_filtered(params.get("host").and_then(Value::as_str));
            Ok(json!({
                "menus": menus.iter().map(|menu| menu_json(*menu)).collect::<Vec<_>>(),
                "hosts": settings.hosts(),
            }))
        }),
        "page.list" => with_settings(session, |settings| {
            let menus = settings
                .menus_filtered(params.get("host").and_then(Value::as_str));
            Ok(json!({
                "pages": menus.iter().map(|menu| page_json(*menu)).collect::<Vec<_>>(),
                "hosts": settings.hosts(),
            }))
        }),
        "page.describe" => with_settings(session, |settings| {
            let file = string_param(&params, "file");
            settings
                .describe_page(&file)
                .map(|controls| json!({ "controls": controls }))
                .map_err(|error| (-32011, error))
        }),
        "control.get" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            settings
                .record(&path)
                .map(|record| record_json(record))
                .ok_or_else(|| (-32011, format!("unknown option path: {path}")))
        }),
        "control.describe" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            settings
                .describe(&path)
                .map_err(|error| (-32011, error))
        }),
        "control.set" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let value = string_param(&params, "value");
            settings
                .set(&path, value)
                .map(|_| json!({ "staged": true }))
                .map_err(|error| (-32011, error))
        }),
        "control.setTyped" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let record = settings
                .record(&path)
                .cloned()
                .ok_or_else(|| (-32011, format!("unknown option path: {path}")))?;
            let requested = string_param(&params, "widget");
            let widget = render::parse_widget(&requested)
                .unwrap_or_else(|| render::choose_widget(&record, None));
            let value = params.get("value").cloned().unwrap_or(Value::Null);
            let literal =
                render::serialize(&record, widget, &value).map_err(|error| (-32011, error))?;
            settings
                .set(&path, literal.clone())
                .map_err(|error| (-32011, error))?;
            let mut package_update: Option<(String, String)> = None;
            if let Some(kind) = catalog::kind_for_widget(&render::widget_name(widget)) {
                if let (Some(package_path), Some(selected)) =
                    (record.spec.package_option.as_deref(), value.as_str())
                {
                    if package_path != path {
                        if let Some(package) = catalog::package_for(kind, selected) {
                            if settings.set(package_path, package).is_ok() {
                                package_update =
                                    Some((package_path.to_string(), package.to_string()));
                            }
                        }
                    }
                }
            }
            let mut response = json!({ "staged": true, "value": literal });
            if let Some((package_path, package)) = package_update {
                response["packagePath"] = json!(package_path);
                response["packageValue"] = json!(package);
            }
            Ok(response)
        }),
        "control.reset" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            Ok(json!({ "staged": settings.reset(&path) }))
        }),
        "edits.preview" => with_settings(session, |settings| {
            let edits = settings.preview().map_err(|error| (-32012, error))?;
            Ok(json!({
                "edits": edits.iter().map(|edit| json!({
                    "file": edit.file,
                    "start": edit.start,
                    "end": edit.end,
                    "before": edit.before,
                    "after": edit.after,
                })).collect::<Vec<_>>()
            }))
        }),
        "edits.apply" => with_settings(session, |settings| {
            settings
                .apply()
                .map(|files| json!({ "files": files }))
                .map_err(|error| (-32012, error))
        }),
        "option.add" => with_settings(session, |settings| {
            let file = string_param(&params, "file");
            let origin = string_param(&params, "origin");
            let path = string_param(&params, "path");
            let widget = params
                .get("widget")
                .and_then(Value::as_str)
                .unwrap_or("text");
            let label = string_param(&params, "label");
            let value = string_param(&params, "value");
            settings
                .add_option(&file, &origin, &path, widget, &label, &value)
                .map(|_| json!({ "added": true }))
                .map_err(|error| (-32012, error))
        }),
        "catalog.styles" => {
            let kind = string_param(&params, "kind");
            catalog::styles(&kind).map_err(|error| (-32012, error))
        }
        "secureboot.status" => {
            let configured = params
                .get("configured")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Ok(secureboot::status(configured))
        }
        "secureboot.config" => Ok(json!({ "block": secureboot::config_block() })),
        "secureboot.writeConfig" => with_settings(session, |settings| {
            let file = string_param(&params, "file");
            let origin = string_param(&params, "origin");
            secureboot::apply_config(settings, &file, &origin)
                .map(|written| json!({ "written": written }))
                .map_err(|error| (-32012, error))
        }),
        "preview.list" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            preview::list(settings, &path).map_err(|error| (-32011, error))
        }),
        "preview.read" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let option = params.get("option").and_then(Value::as_str);
            preview::read(settings, option, &path).map_err(|error| (-32011, error))
        }),
        "preview.upload" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let name = string_param(&params, "name");
            let data = string_param(&params, "data");
            preview::upload(settings, &path, &name, &data).map_err(|error| (-32011, error))
        }),
        "preview.delete" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let name = string_param(&params, "name");
            preview::delete(settings, &path, &name).map_err(|error| (-32011, error))
        }),
        "users.hashPassword" => {
            let password = string_param(&params, "password");
            users::hash_password(&password)
                .map(|hash| json!({ "hash": hash }))
                .map_err(|error| (-32012, error))
        }
        "config.get" => serde_json::to_value(WebConfig::load())
            .map_err(|error| (-32012, error.to_string())),
        "config.set" => (|| {
            let config: WebConfig = serde_json::from_value(params.clone())
                .map_err(|error| (-32012, format!("invalid config: {error}")))?;
            config
                .save()
                .map(|_| json!({ "saved": true }))
                .map_err(|error| (-32012, error))
        })(),
        "config.remember" => (|| {
            let flake = string_param(&params, "flake");
            if flake.trim().is_empty() {
                return Err((-32010, "flake is required".to_string()));
            }
            let mut config = WebConfig::load();
            config.remember(flake.trim());
            config
                .save()
                .map(|_| json!({ "remembered": true }))
                .map_err(|error| (-32012, error))
        })(),
        "apply.run" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let value = string_param(&params, "value");
            let record = settings
                .record(&path)
                .cloned()
                .ok_or_else(|| (-32011, format!("unknown option path: {path}")))?;
            apply::run_apply(&settings.root, &record, &value)
                .map(|_| json!({ "applied": true }))
                .map_err(|error| (-32012, error))
        }),
        "rebuild.run" => with_settings(session, |settings| {
            apply::rebuild(&settings.root)
                .map(|output| json!({ "output": output }))
                .map_err(|error| (-32012, error))
        }),
        "git.status" => with_settings(session, |settings| {
            git::status(&settings.root)
                .and_then(|status| serde_json::to_value(status).map_err(|error| error.to_string()))
                .map_err(|error| (-32012, error))
        }),
        "git.diff" => with_settings(session, |settings| {
            git::diff(&settings.root)
                .map(|diff| json!({ "diff": diff }))
                .map_err(|error| (-32012, error))
        }),
        "git.pull" => with_settings(session, |settings| {
            git::pull_ff_only(&settings.root)
                .map(|_| json!({ "pulled": true }))
                .map_err(|error| (-32012, error))
        }),
        "git.sync" => with_settings(session, |settings| {
            let message = string_param(&params, "message");
            git::sync(&settings.root, &message)
                .map(|_| json!({ "synced": true }))
                .map_err(|error| (-32012, error))
        }),
        "git.commit" => with_settings(session, |settings| {
            let message = string_param(&params, "message");
            git::commit(&settings.root, &message)
                .map(|commit| json!({ "commit": commit }))
                .map_err(|error| (-32012, error))
        }),
        "git.push" => with_settings(session, |settings| {
            git::push(&settings.root)
                .map(|upstream| json!({ "pushed": upstream }))
                .map_err(|error| (-32012, error))
        }),
        "flake.inputs" => with_settings(session, |settings| {
            flake::inputs(&settings.root, settings.declared_inputs())
                .map_err(|error| (-32012, error))
        }),
        "flake.input.addAll" => with_settings(session, |settings| {
            let declared = settings.declared_inputs().to_vec();
            let added = flake::add_all(&settings.root, &declared)
                .map_err(|error| (-32012, error))?;
            settings.reload().map_err(|error| (-32013, error))?;
            Ok(json!({ "added": added }))
        }),
        "flake.input.upsert" => with_settings(session, |settings| {
            let name = string_param(&params, "name");
            let url = string_param(&params, "url");
            let is_flake = params
                .get("flake")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let submodules = params
                .get("submodules")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let lfs = params
                .get("lfs")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let follows: BTreeMap<String, String> = params
                .get("follows")
                .and_then(Value::as_object)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key.clone(), value.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let description = params
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string);
            flake::upsert(
                &settings.root,
                &name,
                &url,
                is_flake,
                submodules,
                lfs,
                follows,
                description,
            )
            .map_err(|error| (-32012, error))?;
            settings.reload().map_err(|error| (-32013, error))?;
            Ok(json!({ "saved": true }))
        }),
        "flake.input.delete" => with_settings(session, |settings| {
            let name = string_param(&params, "name");
            flake::delete(&settings.root, &name).map_err(|error| (-32012, error))?;
            settings.reload().map_err(|error| (-32013, error))?;
            Ok(json!({ "deleted": true }))
        }),
        "flake.channels" => Ok(json!(flake::channels())),
        "flake.suggest" => {
            let kind = string_param(&params, "kind");
            let forge = params
                .get("forge")
                .and_then(Value::as_str)
                .unwrap_or("github");
            let host = params.get("host").and_then(Value::as_str);
            let query = string_param(&params, "query");
            let owner = string_param(&params, "owner");
            let repo = string_param(&params, "repo");
            let branch = string_param(&params, "branch");
            flake::suggest(flake::Suggest {
                kind: &kind,
                forge,
                host,
                query: &query,
                owner: &owner,
                repo: &repo,
                branch: &branch,
            })
            .map(|items| json!(items))
            .map_err(|error| (-32012, error))
        }
        "flake.update.check" => with_settings(session, |settings| {
            let names: Vec<String> = params
                .get("names")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            flake::check_updates(&settings.root, &names).map_err(|error| (-32012, error))
        }),
        "flake.update" => with_settings(session, |settings| {
            let names: Vec<String> = params
                .get("names")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let updated = flake::update(&settings.root, &names).map_err(|error| (-32012, error))?;
            settings.reload().map_err(|error| (-32013, error))?;
            Ok(json!({ "updated": updated }))
        }),
        "file.templates" => with_settings(session, |settings| {
            Ok(authoring::templates_json(&settings.root))
        }),
        "structure.add" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let key = string_param(&params, "key");
            let mode = params.get("mode").and_then(|mode| {
                Some((
                    mode.get("width")?.as_u64()? as u32,
                    mode.get("height")?.as_u64()? as u32,
                    mode.get("refresh")?.as_f64()?,
                ))
            });
            settings
                .add_child(&path, &key, mode)
                .map(|_| json!({ "added": true }))
                .map_err(|error| (-32012, error))
        }),
        "structure.remove" => with_settings(session, |settings| {
            let parent = string_param(&params, "parent");
            let child = string_param(&params, "child");
            settings
                .remove_child(&parent, &child)
                .map(|_| json!({ "removed": true }))
                .map_err(|error| (-32012, error))
        }),
        "structure.rename" => with_settings(session, |settings| {
            let parent = string_param(&params, "parent");
            let child = string_param(&params, "child");
            let key = string_param(&params, "key");
            settings
                .rename_child(&parent, &child, &key)
                .map(|_| json!({ "renamed": true }))
                .map_err(|error| (-32012, error))
        }),
        "monitor.detect" => Ok(nix_web_gui_engine::structure::detect_monitors()),
        "insert.apply" => with_settings(session, |settings| {
            let file = string_param(&params, "file");
            let name = string_param(&params, "name");
            let offset = params
                .get("offset")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            let placeholders: Vec<(String, String)> = params
                .get("placeholders")
                .and_then(Value::as_object)
                .map(|values| {
                    values
                        .iter()
                        .map(|(key, value)| {
                            (
                                key.clone(),
                                value.as_str().unwrap_or_default().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            settings
                .insert_snippet(&file, &name, offset, &placeholders)
                .map(|_| json!({ "inserted": true }))
                .map_err(|error| (-32012, error))
        }),
        "file.add" => with_settings(session, |settings| {
            let path = string_param(&params, "path");
            let kind = params
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("nixos");
            let module_name = string_param(&params, "moduleName");
            let placeholders: Vec<(String, String)> = params
                .get("placeholders")
                .and_then(Value::as_object)
                .map(|values| {
                    values
                        .iter()
                        .map(|(key, value)| {
                            (
                                key.clone(),
                                value.as_str().unwrap_or_default().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            let template_ref = params.get("template");
            let body = if let Some(index) = template_ref.and_then(Value::as_u64) {
                let templates = authoring::load_templates(&settings.root);
                let template = templates
                    .get(index as usize)
                    .ok_or_else(|| (-32011, "unknown template".to_string()))?;
                authoring::render_template(template, &placeholders)
            } else {
                let displayname = std::path::Path::new(&path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "New menu".to_string());
                authoring::template_for(
                    &displayname,
                    kind,
                    &module_name,
                    std::path::Path::new(&path),
                )
            };

            let written =
                authoring::write_new(&settings.root, &path, &body).map_err(|error| (-32012, error))?;
            settings.reload().map_err(|error| (-32013, error))?;
            let flake = std::path::Path::new(&settings.root).join("flake.nix");
            let reminder = if flake.is_file() {
                std::fs::read_to_string(&flake)
                    .map(|text| !text.contains("import-tree"))
                    .unwrap_or(false)
            } else {
                false
            };
            Ok(json!({
                "path": written.display().to_string(),
                "reminder": if reminder { "Add the module to the flake imports" } else { "" },
            }))
        }),
        other => Err((-32601, format!("unknown method: {other}"))),
    };

    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }),
    }
}

fn open_settings(session: &mut Option<SettingsSession>, params: &Value) -> Result<Value, (i32, String)> {
    let root = string_param(params, "root");
    if root.is_empty() {
        return Err((-32010, "root is required".to_string()));
    }
    let settings = SettingsSession::open(root).map_err(|error| (-32010, error))?;
    let menus = settings.menus().len();
    let records = settings.records().count();
    *session = Some(settings);
    Ok(json!({
        "apiVersion": nix_web_gui_engine::API_VERSION,
        "menus": menus,
        "records": records,
    }))
}

fn with_settings<F>(
    session: &mut Option<SettingsSession>,
    handler: F,
) -> Result<Value, (i32, String)>
where
    F: FnOnce(&mut SettingsSession) -> Result<Value, (i32, String)>,
{
    let settings = session
        .as_mut()
        .ok_or_else(|| (-32000, "no open session".to_string()))?;
    handler(settings)
}

fn string_param(params: &Value, name: &str) -> String {
    params
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
