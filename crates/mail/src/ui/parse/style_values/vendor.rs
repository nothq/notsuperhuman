pub(super) fn is_ignored_vendor_property(property: &str) -> bool {
    property.starts_with("mso-")
        || property.starts_with("-webkit-")
        || property.starts_with("-moz-")
        || property.starts_with("-ms-")
}
