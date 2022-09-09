use crate::{ip::NrfSockAddr, Error};
use arrayvec::ArrayString;
use core::str::FromStr;
use no_std_net::{IpAddr, SocketAddr};

pub fn get_host_by_name(hostname: &str) -> Result<IpAddr, Error> {
    #[cfg(feature = "defmt")]
    defmt::debug!("Resolving dns hostname for \"{}\"", hostname);

    if let Ok(ip) = hostname.parse() {
        return Ok(ip);
    }

    if !hostname.is_ascii() {
        return Err(Error::HostnameNotAscii);
    }

    let mut found_ip = None;

    unsafe {
        let hints = nrfxlib_sys::nrf_addrinfo {
            ai_family: nrfxlib_sys::NRF_AF_UNSPEC as _,
            ai_socktype: nrfxlib_sys::NRF_SOCK_STREAM as _,

            ai_flags: 0,
            ai_protocol: 0,
            ai_addrlen: 0,
            ai_addr: core::ptr::null_mut(),
            ai_canonname: core::ptr::null_mut(),
            ai_next: core::ptr::null_mut(),
        };

        let mut result: *mut nrfxlib_sys::nrf_addrinfo = core::ptr::null_mut();

        // A hostname should at most be 256 chars, but we have a null char as well, so we add one
        let mut hostname =
            ArrayString::<257>::from_str(hostname).map_err(|_| Error::HostnameTooLong)?;
        hostname.push('\0');

        let err = nrfxlib_sys::nrf_getaddrinfo(
            hostname.as_ptr(),
            core::ptr::null(),
            &hints as *const _,
            &mut result as *mut *mut _,
        );

        if err > 0 {
            return Err(Error::NrfError(err));
        } else if err == -1 {
            return Err(Error::NrfError(crate::ffi::get_last_error()));
        }

        if result.is_null() {
            return Err(Error::AddressNotFound);
        }

        let mut result_iter = result;

        while !result_iter.is_null() && found_ip.is_none() {
            let address = (*result_iter).ai_addr;

            if (*address).sa_family == nrfxlib_sys::NRF_AF_INET as i32 {
                let dns_addr: &nrfxlib_sys::nrf_sockaddr_in =
                    &*(address as *const nrfxlib_sys::nrf_sockaddr_in);

                let socket_addr: SocketAddr = NrfSockAddr::from(*dns_addr).into();
                found_ip = Some(socket_addr.ip());
            } else if (*address).sa_family == nrfxlib_sys::NRF_AF_INET6 as i32 {
                let dns_addr: &nrfxlib_sys::nrf_sockaddr_in6 =
                    &*(address as *const nrfxlib_sys::nrf_sockaddr_in6);

                let socket_addr: SocketAddr = NrfSockAddr::from(*dns_addr).into();
                found_ip = Some(socket_addr.ip());
            }

            result_iter = (*result_iter).ai_next;
        }

        nrfxlib_sys::nrf_freeaddrinfo(result);

        if let Some(found_ip) = found_ip {
            Ok(found_ip)
        } else {
            Err(Error::AddressNotFound)
        }
    }
}
