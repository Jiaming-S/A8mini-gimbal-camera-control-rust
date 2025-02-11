#![allow(non_snake_case)]

use log::{error, info};
use std::error::Error;
use bincode::deserialize;
use tokio::{net::UdpSocket, time::timeout};

pub mod checksum;
pub mod constants;
pub mod control;

#[derive(Debug)]
pub struct A8Mini {
	pub command_socket: UdpSocket,
	pub http_socket: UdpSocket,
}

impl A8Mini {
	pub async fn connect() -> Result<Self, Box<dyn Error>> {
		Ok(Self::connect_to(constants::CAMERA_IP, constants::CAMERA_COMMAND_PORT, constants::CAMERA_HTTP_PORT, "8080", "8088").await?)
	}

	pub async fn connect_to(camera_ip: &str, camera_command_port: &str, camera_http_port: &str, local_command_port: &str, local_http_port: &str) -> Result<A8Mini, Box<dyn Error>> {
		let camera: A8Mini = A8Mini {
			command_socket: UdpSocket::bind(format!("0.0.0.0:{}", local_command_port)).await?,
			http_socket: UdpSocket::bind(format!("0.0.0.0:{}", local_http_port)).await?,
		};

		camera.command_socket.connect(format!("{}:{}", camera_ip, camera_command_port)).await?;
		camera.http_socket.connect(format!("{}:{}", camera_ip, camera_http_port)).await?;
		Ok(camera)
	}

	pub async fn send_command_blind<T: control::Command>(&self, command: T) -> Result<(), Box<dyn Error>> {
		info!("[COMMAND] Sending command with bytes: {:?}", command.to_bytes());
		info!("[COMMAND] Sending command with DATA_LEN: {:?} | CMD_ID: {:?}", command.to_bytes()[3], command.to_bytes()[7]);

		if self.command_socket.send(command.to_bytes().as_slice()).await? == 0 {
			error!("[COMMAND] No bytes sent.");
			return Err("No bytes sent.".into());
		}

		info!("[COMMAND] Command sent successfully.");

		Ok(())
	}

	pub async fn send_command<T: control::Command>(&self, command: T) -> Result<[u8; constants::RECV_BUFF_SIZE], Box<dyn Error>> {
		self.send_command_blind(command).await?;
		let mut recv_buffer = [0; constants::RECV_BUFF_SIZE];

		info!("[COMMAND] Waiting for response.");

		let recv_len = timeout(constants::RECV_TIMEOUT, self.command_socket.recv(&mut recv_buffer)).await??;
		if recv_len == 0  {
			error!("[COMMAND] No bytes received.");
			return Err("No bytes received.".into());
		}

		info!("[COMMAND] Response of size {} received successfully: {:?}", recv_len, recv_buffer);
		Ok(recv_buffer)
	}

	pub async fn get_attitude_information(&self) -> Result<control::A8MiniAtittude, Box<dyn Error>> {
		let attitude_bytes = self.send_command(control::A8MiniSimpleCommand::AttitudeInformation).await?;
		let attitude_info: control::A8MiniAtittude = deserialize(&attitude_bytes)?;
		Ok(attitude_info)
	}

	pub async fn send_http_query_blind<T: control::HTTPQuery>(&self, query: T) -> Result<(), Box<dyn Error>> {
		info!("[HTTP] Sending query with content: {:?}", query.to_string());

		if self.http_socket.send(query.to_string().as_ref()).await? == 0 {
			error!("[HTTP] No bytes sent.");
			return Err("No bytes sent.".into());
		}

		info!("[HTTP] Query sent successfully.");
		Ok(())
	}

	pub async fn send_http_query<T: control::HTTPQuery>(&self, query: T) -> Result<String, Box<dyn Error>> {
		self.send_http_query_blind(query).await?;
		let mut recv_buffer = [0; 256];

		info!("[HTTP] Waiting for response.");

		let recv_len = timeout(constants::RECV_TIMEOUT, self.http_socket.recv(&mut recv_buffer)).await??;
		if recv_len == 0  {
			error!("[HTTP] No response received.");
			return Err("No response received.".into());
		}

		info!("[HTTP] Response of size {} received successfully: {:?}", recv_len, recv_buffer);
		
		Ok(String::from_utf8(recv_buffer.to_vec())?)
	}
}



#[cfg(test)]
mod tests {
	use super::*;
	use std::thread::sleep;
	use std::time::Duration;

	#[tokio::test]
	async fn test_control_lock()  -> Result<(), Box<dyn Error>> {
		let cam: A8Mini = A8Mini::connect().await?;

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(900, 0)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(900, -900)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(900, 250)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(-900, 0)).await?;
		sleep(Duration::from_millis(2500));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(-900, -900)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(-900, 250)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniSimpleCommand::AutoCenter).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_take_and_download_photo()  -> Result<(), Box<dyn Error>> {
		let cam: A8Mini = A8Mini::connect().await?;

		cam.send_command_blind(control::A8MiniSimpleCommand::TakePicture).await?;
		sleep(Duration::from_millis(500));

		Ok(())
	}

	#[tokio::test]
	async fn test_send_simple_commands_blind() -> Result<(), Box<dyn Error>> {
		let cam: A8Mini = A8Mini::connect().await?;

		cam.send_command_blind(control::A8MiniSimpleCommand::RotateLeft).await?;
		sleep(Duration::from_millis(500));
		
		cam.send_command_blind(control::A8MiniSimpleCommand::RotateRight).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniSimpleCommand::RotateLeft).await?;
		sleep(Duration::from_millis(500));

		cam.send_command_blind(control::A8MiniSimpleCommand::StopRotation).await?;

		cam.send_command_blind(control::A8MiniSimpleCommand::RotateUp).await?;
		sleep(Duration::from_millis(500));

		cam.send_command_blind(control::A8MiniSimpleCommand::RotateDown).await?;
		sleep(Duration::from_millis(500));

		cam.send_command_blind(control::A8MiniSimpleCommand::StopRotation).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniSimpleCommand::AutoCenter).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_send_complex_commands_blind() -> Result<(), Box<dyn Error>> {
		let cam: A8Mini = A8Mini::connect().await?;

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchSpeed(50, 50)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchSpeed(50, 10)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchSpeed(-25, -15)).await?;
		sleep(Duration::from_millis(6000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchSpeed(0, 0)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(90, 0)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(90, -90)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(-90, -90)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(-90, 0)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniComplexCommand::SetYawPitchAngle(0, 0)).await?;
		sleep(Duration::from_millis(1000));

		cam.send_command_blind(control::A8MiniSimpleCommand::AutoCenter).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_send_command_with_ack() -> Result<(), Box<dyn Error>> {
		let cam: A8Mini = A8Mini::connect().await?;
		cam.get_attitude_information().await?;
		Ok(())
	}
}
