import * as net from 'net';
import { IpcRequest, IpcResponse, ProcessesResponse, SummaryResponse } from './types';

export class OitClient {
  constructor(private socketPath: string) {}

  async send(command: string, args: Record<string, unknown> = {}): Promise<IpcResponse> {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(this.socketPath);
      let data = '';

      const cleanup = () => {
        socket.removeAllListeners();
        socket.destroy();
      };

      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error('Connection timeout'));
      }, 5000);

      socket.on('connect', () => {
        const request: IpcRequest = { command, args };
        socket.write(JSON.stringify(request) + '\n');
      });

      socket.on('data', (chunk) => {
        data += chunk.toString();
        if (data.includes('\n')) {
          clearTimeout(timeout);
          cleanup();
          try {
            resolve(JSON.parse(data.trim()));
          } catch (e) {
            reject(new Error('Invalid JSON response'));
          }
        }
      });

      socket.on('error', (err) => {
        clearTimeout(timeout);
        cleanup();
        reject(err);
      });

      socket.on('close', () => {
        clearTimeout(timeout);
        if (!data.includes('\n')) {
          reject(new Error('Connection closed before response'));
        }
      });
    });
  }

  async ping(): Promise<boolean> {
    try {
      const response = await this.send('ping');
      return response.success;
    } catch {
      return false;
    }
  }

  async processes(): Promise<ProcessesResponse | null> {
    try {
      const response = await this.send('processes');
      if (response.success && response.result) {
        return response.result as ProcessesResponse;
      }
      return null;
    } catch {
      return null;
    }
  }

  async summary(): Promise<SummaryResponse | null> {
    try {
      const response = await this.send('summary');
      if (response.success && response.result) {
        return response.result as SummaryResponse;
      }
      return null;
    } catch {
      return null;
    }
  }

  async restart(name?: string): Promise<IpcResponse> {
    return this.send('restart', name ? { name } : {});
  }

  async kill(name: string): Promise<IpcResponse> {
    return this.send('kill', { name });
  }

  async start(name: string): Promise<IpcResponse> {
    return this.send('start', { name });
  }
}
