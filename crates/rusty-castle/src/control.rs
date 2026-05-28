//! SOAP control dispatch for the initial MediaServer.

use crate::catalog::{MediaContainer, MediaItem, StaticCatalog};
use dlna_core::ProtocolInfoRef;
use log::{debug, warn};
use upnp_av_core::connection_manager::{
    protocol_info_list_to_string, write_current_connection_ids_response,
    write_get_protocol_info_response,
};
use upnp_av_core::content_directory::{
    BrowseFlag, BrowseResponseRef, write_browse_response, write_system_update_id_response,
};
use upnp_av_core::didl::{ObjectRef, ResourceRef, UpnpClass, didl_to_string};
use upnp_core::soap::{ActionRequestRef, ResponseArgumentRef, parse_action_request};

/// A SOAP control response body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlResponse {
    /// XML response body.
    pub body: String,
    /// HTTP status code to use for the SOAP response.
    pub status_code: u16,
}

/// Control dispatch errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlError {
    /// SOAP XML could not be parsed.
    BadRequest,
    /// The requested action or service is not implemented.
    InvalidAction,
    /// The target object does not exist.
    NoSuchObject,
    /// XML writing failed.
    WriteFailed,
}

impl ControlError {
    /// Converts this error into a SOAP fault response.
    pub fn into_response(self) -> ControlResponse {
        let (code, desc) = match self {
            Self::BadRequest => (402, "Invalid Args"),
            Self::InvalidAction => (401, "Invalid Action"),
            Self::NoSuchObject => (701, "No Such Object"),
            Self::WriteFailed => (501, "Action Failed"),
        };
        let mut body = String::new();
        let _ = upnp_core::soap::write_fault(&mut body, code, desc);
        ControlResponse {
            body,
            status_code: 500,
        }
    }
}

/// Handles a SOAP control request for ContentDirectory or ConnectionManager.
pub fn handle_control(
    soap_xml: &str,
    catalog: &StaticCatalog,
) -> Result<ControlResponse, ControlError> {
    let action = parse_action_request(soap_xml).map_err(|err| {
        warn!("failed to parse soap control request: {err}");
        ControlError::BadRequest
    })?;
    debug!(
        "soap action service={} action={}",
        action.service_type.kind, action.action_name
    );
    match action.service_type.kind {
        "ContentDirectory" => handle_content_directory(&action, catalog),
        "ConnectionManager" => handle_connection_manager(&action, catalog),
        _ => {
            warn!("unsupported soap service={}", action.service_type.kind);
            Err(ControlError::InvalidAction)
        }
    }
}

fn handle_content_directory(
    action: &ActionRequestRef<'_>,
    catalog: &StaticCatalog,
) -> Result<ControlResponse, ControlError> {
    match action.action_name {
        "Browse" => browse(action, catalog),
        "GetSystemUpdateID" => {
            let mut body = String::new();
            write_system_update_id_response(&mut body, catalog.update_id())
                .map_err(|_| ControlError::WriteFailed)?;
            Ok(ok(body))
        }
        "GetSearchCapabilities" => empty_cd_response("GetSearchCapabilities", "SearchCaps"),
        "GetSortCapabilities" => empty_cd_response("GetSortCapabilities", "SortCaps"),
        _ => Err(ControlError::InvalidAction),
    }
}

fn handle_connection_manager(
    action: &ActionRequestRef<'_>,
    catalog: &StaticCatalog,
) -> Result<ControlResponse, ControlError> {
    match action.action_name {
        "GetProtocolInfo" => {
            let infos = collect_protocol_info(catalog);
            let source =
                protocol_info_list_to_string(&infos).map_err(|_| ControlError::WriteFailed)?;
            let mut body = String::new();
            write_get_protocol_info_response(&mut body, "", &source)
                .map_err(|_| ControlError::WriteFailed)?;
            Ok(ok(body))
        }
        "GetCurrentConnectionIDs" => {
            let mut body = String::new();
            write_current_connection_ids_response(&mut body)
                .map_err(|_| ControlError::WriteFailed)?;
            Ok(ok(body))
        }
        _ => Err(ControlError::InvalidAction),
    }
}

fn browse(
    action: &ActionRequestRef<'_>,
    catalog: &StaticCatalog,
) -> Result<ControlResponse, ControlError> {
    let object_id = arg(action, "ObjectID").unwrap_or("0");
    let flag = arg(action, "BrowseFlag")
        .and_then(BrowseFlag::parse)
        .unwrap_or(BrowseFlag::DirectChildren);
    debug!(
        "browse request object_id={} flag={:?} catalog_items={}",
        object_id,
        flag,
        catalog.items().len()
    );
    let (result, number_returned, total_matches) = match (object_id, flag) {
        ("0", BrowseFlag::Metadata) => (root_didl(catalog)?, 1, 1),
        ("0", BrowseFlag::DirectChildren) => {
            let didl = children_didl(catalog, "0")?;
            let count = catalog.child_count("0");
            (didl, count, count)
        }
        (id, BrowseFlag::Metadata) => {
            if let Some(container) = catalog.container(id) {
                (container_didl(catalog, container)?, 1, 1)
            } else {
                let item = catalog.item(id).ok_or(ControlError::NoSuchObject)?;
                (item_didl(item)?, 1, 1)
            }
        }
        (id, BrowseFlag::DirectChildren) => {
            if catalog.container(id).is_some() {
                let didl = children_didl(catalog, id)?;
                let count = catalog.child_count(id);
                (didl, count, count)
            } else if catalog.item(id).is_some() {
                (empty_didl()?, 0, 0)
            } else {
                return Err(ControlError::NoSuchObject);
            }
        }
    };
    let mut body = String::new();
    write_browse_response(
        &mut body,
        &BrowseResponseRef {
            result: &result,
            number_returned,
            total_matches,
            update_id: catalog.update_id(),
        },
    )
    .map_err(|_| ControlError::WriteFailed)?;
    Ok(ok(body))
}

fn arg<'a>(action: &'a ActionRequestRef<'a>, name: &str) -> Option<&'a str> {
    action
        .arguments()
        .find(|arg| arg.name == name)
        .map(|arg| arg.value)
}

fn root_didl(catalog: &StaticCatalog) -> Result<String, ControlError> {
    didl_to_string(&[ObjectRef {
        id: "0",
        parent_id: "-1",
        restricted: true,
        title: "Media",
        class: UpnpClass::Container,
        child_count: Some(catalog.child_count("0")),
        resources: &[],
    }])
    .map_err(|_| ControlError::WriteFailed)
}

fn children_didl(catalog: &StaticCatalog, parent_id: &str) -> Result<String, ControlError> {
    let resources: Vec<[ResourceRef<'_>; 1]> = catalog
        .child_items(parent_id)
        .map(|item| [resource_for_item(item)])
        .collect();
    let mut objects: Vec<ObjectRef<'_>> = catalog
        .child_containers(parent_id)
        .map(|container| object_for_container(catalog, container))
        .collect();
    objects.extend(
        catalog
            .child_items(parent_id)
            .zip(resources.iter())
            .map(|(item, res)| object_for_item(item, res)),
    );
    didl_to_string(&objects).map_err(|_| ControlError::WriteFailed)
}

fn container_didl(
    catalog: &StaticCatalog,
    container: &MediaContainer,
) -> Result<String, ControlError> {
    didl_to_string(&[object_for_container(catalog, container)])
        .map_err(|_| ControlError::WriteFailed)
}

fn item_didl(item: &MediaItem) -> Result<String, ControlError> {
    let resources = [resource_for_item(item)];
    didl_to_string(&[object_for_item(item, &resources)]).map_err(|_| ControlError::WriteFailed)
}

fn empty_didl() -> Result<String, ControlError> {
    didl_to_string(&[]).map_err(|_| ControlError::WriteFailed)
}

fn resource_for_item(item: &MediaItem) -> ResourceRef<'_> {
    ResourceRef {
        url: &item.url,
        protocol_info: item.protocol_info,
        size: item.size,
        duration: item.duration.as_deref(),
    }
}

fn object_for_item<'a>(item: &'a MediaItem, resources: &'a [ResourceRef<'a>]) -> ObjectRef<'a> {
    ObjectRef {
        id: &item.id,
        parent_id: &item.parent_id,
        restricted: true,
        title: &item.title,
        class: UpnpClass::VideoItem,
        child_count: None,
        resources,
    }
}

fn object_for_container<'a>(
    catalog: &StaticCatalog,
    container: &'a MediaContainer,
) -> ObjectRef<'a> {
    ObjectRef {
        id: &container.id,
        parent_id: &container.parent_id,
        restricted: true,
        title: &container.title,
        class: UpnpClass::Container,
        child_count: Some(catalog.child_count(&container.id)),
        resources: &[],
    }
}

fn empty_cd_response(
    action_name: &str,
    arg_name: &'static str,
) -> Result<ControlResponse, ControlError> {
    let mut body = String::new();
    upnp_core::soap::write_action_response(
        &mut body,
        &upnp_av_core::content_directory::CONTENT_DIRECTORY_SERVICE,
        action_name,
        &[ResponseArgumentRef {
            name: arg_name,
            value: "",
        }],
    )
    .map_err(|_| ControlError::WriteFailed)?;
    Ok(ok(body))
}

fn collect_protocol_info(catalog: &StaticCatalog) -> Vec<ProtocolInfoRef<'static>> {
    let mut infos = Vec::new();
    for item in catalog.items() {
        if !infos.contains(&item.protocol_info) {
            infos.push(item.protocol_info);
        }
    }
    if infos.is_empty() {
        infos.push(ProtocolInfoRef::sony_mp4());
    }
    infos
}

fn ok(body: String) -> ControlResponse {
    ControlResponse {
        body,
        status_code: 200,
    }
}
