
use std::net::{SocketAddr, ToSocketAddrs};
use url::Url;

use crate::{AuthSpec, Spec, SpecKind};

xflags::xflags! {
    cmd args {
        /// The login for the user with the necessary permissions
        required login_name: String
        cmd token {
            /// The oauth access token, if you already have it
            required token: String
        }
        cmd get_token {
            required app_id: String
            required app_secret: String
            /// Address to use for local server. Needs to match the one set in the Twitch dev console.
            required address: String
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidAddress(String),
    UrlParse(url::ParseError),
    Io(std::io::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use Error::*;
        match self {
            InvalidAddress(non_address) => write!(f, "\"{non_address}\" is not a valid address."),
            UrlParse(_) => write!(f, "Url parse error"),
            Io(_) => write!(f, "I/O error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use Error::*;
        match self {
            InvalidAddress(_) => None,
            UrlParse(e) => Some(e),
            Io(e) => Some(e),
        }
    }
}

impl Args {
    pub fn to_spec(self) -> Result<Spec, Error> {
        let kind = match self.subcommand {
            ArgsCmd::Token(Token{ token }) => {
                SpecKind::Token(token)
            }
            ArgsCmd::Get_token(Get_token {
                app_id,
                app_secret,
                address,
            }) => {
                let addr = {
                    fn first_addr(to_addrs: impl ToSocketAddrs) -> Option<SocketAddr> {
                        to_addrs.to_socket_addrs().ok()?.next()
                    }
            
                    let addr_vec = Url::parse(&address)
                        .map_err(Error::UrlParse)?
                        .socket_addrs(|| None)
                        .map_err(Error::Io)?;
            
                    if let Some(addr) = first_addr(&*addr_vec) {
                        Some(addr)
                    } else {
                        first_addr((address.as_str(), 8080))
                    }
                };

                let Some(addr) = addr else {
                    return Err(Error::InvalidAddress(address))
                };

                SpecKind::Auth(
                    AuthSpec {
                        addr,
                        addr_string: address,
                        app_id,
                        app_secret,
                    }
                )
            }
        };

        Ok(Spec {
            login_name: self.login_name,
            kind,
        })
    }
}


    
