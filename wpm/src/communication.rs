use crate::SocketMessage;
use interprocess::local_socket::traits::Stream as StreamExt;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::Stream;
use interprocess::local_socket::ToNsName;
use std::io::Write;

pub fn send_message(to: &str, message: SocketMessage) -> Result<(), std::io::Error> {
    let json = serde_json::to_string(&message)?;
    let name = to.to_ns_name::<GenericNamespaced>()?;
    let connection = Stream::connect(name)?;
    let (_, mut sender) = connection.split();
    sender.write_all(json.as_bytes())?;

    Ok(())
}

pub fn send_str(to: &str, message: &str) -> Result<(), std::io::Error> {
    let name = to.to_ns_name::<GenericNamespaced>()?;
    let connection = Stream::connect(name)?;
    let (_, mut sender) = connection.split();
    sender.write_all(message.as_bytes())?;

    Ok(())
}
