// standard
use std::convert::TryFrom;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::os::raw::c_char;
use std::str::FromStr;

// extern
use anyhow::bail;
#[cfg(feature = "impl-lib")]
use cgosling_proc_macros::*;
use tor_interface::tor_provider::{CircuitToken, DomainAddr, OnionAddr, OnionAddrV3, OnionStream, TargetAddr};

// internal
use crate::context::*;
use crate::crypto::*;
use crate::error::*;
use crate::ffi::*;
use crate::macros::*;

/// The maximum number of bytes needed to store a target address
/// in the format domainname:port (including null-terminator)
/// Maximum length of a human-readbale domain name is 253 bytes (per RFC 1035)
/// see: https://stackoverflow.com/a/32294443
/// Maximum length of the :port section is 6 bytes
/// null-terminator is 1 byte
pub const TARGET_ADDRESS_STRING_SIZE: usize = 260;

/// An internet socket address, either IPv4 or IPv6
pub struct GoslingIpAddress;
define_registry! {IpAddr}

/// An endpoint to connect to over tor
pub struct GoslingTargetAddress;
define_registry! {TargetAddr}

/// A stream isolation token
pub type GoslingCircuitToken = usize;

//
// Free Functions
//

/// Frees a gosling_ip_address object
///
/// @param in_ip_address: the ip address to free
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub extern "C" fn gosling_ip_address_free(in_ip_address: *mut GoslingIpAddress) {
    impl_registry_free!(in_ip_address, IpAddr);
}

/// Frees a gosling_target_address object
///
/// @param in_target_address: the target address to free
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub extern "C" fn gosling_target_address_free(in_target_address: *mut GoslingTargetAddress) {
    impl_registry_free!(in_target_address, TargetAddr);
}

//
// Clone Functions
//

/// Copy method for gosling_ip_address
///
/// @param out_ip_address: returned copy
/// @param ip_address: original to copy
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_ip_address_clone(
    out_ip_address: *mut *mut GoslingIpAddress,
    ip_address: *const GoslingIpAddress,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_ip_address);
        ensure_not_null!(ip_address);

        let ip_address = match get_ip_addr_registry().get(ip_address as usize) {
            Some(ip_address) => ip_address.clone(),
            None => bail_invalid_handle!(ip_address),
        };
        let handle = get_ip_addr_registry().insert(ip_address);
        *out_ip_address = handle as *mut GoslingIpAddress;

        Ok(())
    })
}

/// Copy method for gosling_target_address
///
/// @param out_target_address: returned copy
/// @param target_address: original to copy
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_clone(
    out_target_address: *mut *mut GoslingTargetAddress,
    target_address: *const GoslingTargetAddress,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_target_address);
        ensure_not_null!(target_address);

        let target_address = match get_target_addr_registry().get(target_address as usize) {
            Some(target_address) => target_address.clone(),
            None => bail_invalid_handle!(target_address),
        };
        let handle = get_target_addr_registry().insert(target_address);
        *out_target_address = handle as *mut GoslingTargetAddress;

        Ok(())
    })
}

//
// Connect Method
//

/// Connect to a target address using the provided gosling context's tor provider.
///
/// @param context: the context to use to connect with
/// @param out_tcp_socket: returned connected tcp socket
/// @param target_address: the destination address to connect to
/// @param circuit_token: the circuit isolation token
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub extern "C" fn gosling_context_connect(
    context: *mut GoslingContext,
    out_tcp_socket: *mut GoslingTcpSocket,
    target_address: *const GoslingTargetAddress,
    circuit_token: GoslingCircuitToken,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(context);
        ensure_not_null!(out_tcp_socket);
        ensure_not_null!(target_address);

        let mut context_tuple_registry = get_context_tuple_registry();
        let context = match context_tuple_registry.get_mut(context as usize) {
            Some(context) => context,
            None => bail_invalid_handle!(context),
        };

        let target_address = match get_target_addr_registry().get(target_address as usize) {
            Some(target_address) => target_address.clone(),
            None => bail_invalid_handle!(target_address),
        };

        let onion_stream = context.0.connect(target_address, Some(circuit_token))?;

        unsafe {
            *out_tcp_socket = onion_stream.into_raw();
        }
        Ok(())
    })
}

//
// Ip Address Methods
//

/// Create ip address from four ipv4 octets.
///
/// @param out_ip_address: returned ip address
/// @param a: first octet
/// @param b: second octet
/// @param c: third octet
/// @param d: fourth octet
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_ip_address_from_ipv4(
    out_ip_address: *mut *mut GoslingIpAddress,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_ip_address);

        let ip_addr = Ipv4Addr::new(a, b, c, d);
        let handle = get_ip_addr_registry().insert(ip_addr.into());
        *out_ip_address = handle as *mut GoslingIpAddress;

        Ok(())
    })
}

/// Create target address from eight ipv6 16-bit sgements
///
/// @param out_ip_address: returned ip address
/// @param a: first segment
/// @param b: second segment
/// @param c: third segment
/// @param d: fourth segment
/// @param e: fifth segment
/// @param f: sixth segment
/// @param g: seventh segment
/// @param h: eigth segment
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_ip_address_from_ipv6(
    out_ip_address: *mut *mut GoslingIpAddress,
    a: u16,
    b: u16,
    c: u16,
    d: u16,
    e: u16,
    f: u16,
    g: u16,
    h: u16,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_ip_address);
        let ip_addr = Ipv6Addr::new(a, b, c, d, e, f, g, h);
        let handle = get_ip_addr_registry().insert(ip_addr.into());
        *out_ip_address = handle as *mut GoslingIpAddress;

        Ok(())
    })
}

//
// Target Address Methods
//

/// Create target address from an ip address and a port.
///
/// @param out_target_address: returned target address
/// @param ip_address: target ip address
/// @param port: target port
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_from_ip_address(
    out_target_address: *mut *mut GoslingTargetAddress,
    ip_address: *const GoslingIpAddress,
    port: u16,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_target_address);
        ensure_not_null!(ip_address);

        let ip_address = match get_ip_addr_registry().get(ip_address as usize) {
            Some(ip_address) => ip_address.clone(),
            None => bail_invalid_handle!(ip_address),
        };

        let target_address = TargetAddr::Socket(SocketAddr::new(ip_address, port));

        let handle = get_target_addr_registry().insert(target_address);
        *out_target_address = handle as *mut GoslingTargetAddress;

        Ok(())
    })
}

/// Create target address from domain and port.
/// The resulting target address is in the format domain:port
///
/// @param out_target_address: returned target address
/// @param domain: the target domain
/// @param domain_length: the number of chars in domain not including any null-terminator
/// @param port: the target port
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_from_domain(
    out_target_address: *mut *mut GoslingTargetAddress,
    domain: *const c_char,
    domain_length: usize,
    port: u16,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_target_address);
        ensure_not_null!(domain);
        if domain_length == 0 {
            bail!("domain_length must be greater than 0");
        }

        let domain_view = std::slice::from_raw_parts(domain as *const u8, domain_length);
        let domain_str = std::str::from_utf8(domain_view)?;

        let target_address =
            TargetAddr::Domain(DomainAddr::try_from((domain_str.to_string(), port))?);
        let handle = get_target_addr_registry().insert(target_address);
        *out_target_address = handle as *mut GoslingTargetAddress;

        Ok(())
    })
}

/// Create target address from onion service id and port.
///
/// @param out_target_address: returned target address
/// @param service_id: the target onion service id
/// @param port: the target port
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_from_v3_onion_service_id(
    out_target_address: *mut *mut GoslingTargetAddress,
    service_id: *const GoslingV3OnionServiceId,
    port: u16,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_target_address);
        ensure_not_null!(service_id);

        let service_id = match get_v3_onion_service_id_registry().get(service_id as usize) {
            Some(service_id) => service_id.clone(),
            None => bail_invalid_handle!(service_id),
        };

        let target_address =
            TargetAddr::OnionService(OnionAddr::V3(OnionAddrV3::new(service_id, port)));
        let handle = get_target_addr_registry().insert(target_address);
        *out_target_address = handle as *mut GoslingTargetAddress;

        Ok(())
    })
}

/// Create target address from some string representation
///
/// @param out_target_address: returned target address
/// @param target_address: serialised target address
/// @param target_address_length: the number of chars in string not including any null-terminator
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_from_string(
    out_target_address: *mut *mut GoslingTargetAddress,
    target_address: *const c_char,
    target_address_length: usize,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(out_target_address);
        ensure_not_null!(target_address);
        if target_address_length == 0 {
            bail!("target_address_length must be greater than 0");
        }

        let target_address_view =
            std::slice::from_raw_parts(target_address as *const u8, target_address_length);
        let target_address_str = std::str::from_utf8(target_address_view)?;

        let target_address = TargetAddr::from_str(target_address_str)?;
        let handle = get_target_addr_registry().insert(target_address);
        *out_target_address = handle as *mut GoslingTargetAddress;

        Ok(())
    })
}

/// Write target address to null-terminated string
///
/// @param target_address: the target address to write
/// @param out_target_address_string: buffer to be filled with string
/// @param target_address_string_size: size of the out_string buffer in bytes. The maximum
///  required size is 262 bytes.
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_target_address_to_string(
    target_address: *const GoslingTargetAddress,
    out_target_address_string: *mut c_char,
    target_address_string_size: usize,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(target_address);
        ensure_not_null!(out_target_address_string);

        let target_address_string = match get_target_addr_registry().get(target_address as usize) {
            Some(target_address) => target_address.to_string(),
            None => bail_invalid_handle!(target_address),
        };

        let target_address_string_len = target_address_string.len();
        if target_address_string_len >= target_address_string_size {
            bail!(
                "string_size must be at least '{}', received '{}'",
                target_address_string_len,
                target_address_string_size
            );
        }

        unsafe {
            // copy target_address_string into output buffer
            let target_address_string_view = std::slice::from_raw_parts_mut(
                out_target_address_string as *mut u8,
                target_address_string_size,
            );
            std::ptr::copy(
                target_address_string.as_ptr(),
                target_address_string_view.as_mut_ptr(),
                target_address_string_len,
            );
            // add final null-terminator
            target_address_string_view[target_address_string_len] = 0u8;
        }

        Ok(())
    })
}

//
// Circuit Token Methods
//

/// Generate a circuit token to isolate connect calls
///
/// @param context: the context to use to connect with
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_context_generate_circuit_token(
    context: *mut GoslingContext,
    error: *mut *mut GoslingError,
) -> GoslingCircuitToken {
    translate_failures(!0usize, error, || -> anyhow::Result<CircuitToken> {
        ensure_not_null!(context);

        let mut context_tuple_registry = get_context_tuple_registry();
        let token = match context_tuple_registry.get_mut(context as usize) {
            Some(context) => context.0.generate_circuit_token(),
            None => bail_invalid_handle!(context),
        };
        Ok(token)
    })
}

/// Release a context's circuit token.
///
/// @param context: the context to use to connect with
/// @param circuit_token: circuit token to destroy
/// @param error: filled on error
#[no_mangle]
#[cfg_attr(feature = "impl-lib", rename_impl)]
pub unsafe extern "C" fn gosling_context_release_circuit_token(
    context: *mut GoslingContext,
    circuit_token: GoslingCircuitToken,
    error: *mut *mut GoslingError,
) {
    translate_failures((), error, || -> anyhow::Result<()> {
        ensure_not_null!(context);

        let mut context_tuple_registry = get_context_tuple_registry();
        match context_tuple_registry.get_mut(context as usize) {
            Some(context) => context.0.release_circuit_token(circuit_token),
            None => bail_invalid_handle!(context),
        };
        Ok(())
    })
}
