import type { ChatMessage } from '../types';
import { saveChatSession, loadChatSession, listChatSessions, deleteChatSession } from '$lib/utils/tauri';

export interface SerializedSession {
  id: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export class ChatSession {
  id: string;
  messages: ChatMessage[] = [];
  createdAt: number;
  updatedAt: number;

  constructor(id?: string) {
    this.id = id || `session-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    this.createdAt = Date.now();
    this.updatedAt = Date.now();
  }

  addMessage(message: ChatMessage): void {
    this.messages.push(message);
    this.updatedAt = Date.now();
  }

  getMessages(): ChatMessage[] {
    return [...this.messages];
  }

  replaceMessages(messages: ChatMessage[]): void {
    this.messages = [...messages];
    this.updatedAt = Date.now();
  }

  clear(): void {
    this.messages = [];
    this.updatedAt = Date.now();
  }

  serialize(): string {
    const data: SerializedSession = {
      id: this.id,
      messages: this.messages,
      createdAt: this.createdAt,
      updatedAt: this.updatedAt,
    };
    return JSON.stringify(data);
  }

  static deserialize(json: string): ChatSession {
    const data: SerializedSession = JSON.parse(json);
    const session = new ChatSession(data.id);
    session.messages = data.messages;
    session.createdAt = data.createdAt;
    session.updatedAt = data.updatedAt;
    return session;
  }

  async save(): Promise<void> {
    await saveChatSession(this.id, this.serialize());
  }

  static async load(sessionId: string): Promise<ChatSession> {
    const json = await loadChatSession(sessionId);
    return ChatSession.deserialize(json);
  }

  static async listAll(): Promise<string[]> {
    return listChatSessions();
  }

  static async delete(sessionId: string): Promise<void> {
    await deleteChatSession(sessionId);
  }
}
