

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use url::Url;

use crate::{AuthSpec, Spec, SpecKind, TokenSpec};

xflags::xflags! {
    cmd args {
        /// The login for the user with the necessary permissions
        required login_name: String

        /// The oauth access token, if you already have it. Either this or all of app_id, app_secret, and address are required
        optional --token token: String

        optional --app_id app_id: String
        optional --app_secret app_secret: String
        /// Address to use for local server. Needs to match the one set in the Twitch dev console.
        optional --address address: String

        cmd get_rewards {
        }
        cmd modify_rewards {
            /// Filepath to a .lua file to evaluate to select the modifications to perform
            required --lua lua_path: PathBuf
        }
    }
}

#[derive(Debug)]
pub enum Error {
    AppIdMissing,
    AppSecretMissing,
    AddressMissing,
    InvalidAddress(String),
    UrlParse(url::ParseError),
    Io(std::io::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use Error::*;
        match self {
            AppIdMissing => write!(f, "--app_id flag missing"),
            AppSecretMissing => write!(f, "--app_secret flag missing"),
            AddressMissing => write!(f, "--address flag missing"),
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
            AppIdMissing
            | AppSecretMissing
            | AddressMissing
            | InvalidAddress(_) => None,
            UrlParse(e) => Some(e),
            Io(e) => Some(e),
        }
    }
}

impl Args {
    pub fn to_spec(self) -> Result<Spec, Error> {
        let token_spec = if let Some(token) = self.token {
            TokenSpec::Token(token)
        } else {
            let Some(app_id) = self.app_id else {
                return Err(Error::AppIdMissing)
            };
            let Some(app_secret) = self.app_secret else {
                return Err(Error::AppSecretMissing)
            };
            let Some(address) = self.address else {
                return Err(Error::AddressMissing)
            };
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
    
            TokenSpec::Auth(
                AuthSpec {
                    addr,
                    addr_string: address,
                    app_id,
                    app_secret,
                }
            )
        };

        let kind = match self.subcommand {
            ArgsCmd::Get_rewards(Get_rewards{}) => {
                SpecKind::GetRewards
            }
            ArgsCmd::Modify_rewards(Modify_rewards {
                lua
            }) => {
                SpecKind::ModifyRewards(lua)
            }
        };

        Ok(Spec {
            login_name: self.login_name,
            token_spec,
            kind,
        })
    }
}


    
