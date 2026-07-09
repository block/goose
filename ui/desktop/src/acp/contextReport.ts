import { getAcpClient } from './acpConnection';
import type { ContextReport } from '../types/contextReport';

export async function getContextReport(sessionId: string): Promise<ContextReport> {
  const client = await getAcpClient();
  return client.goose.contextReport_unstable({ sessionId });
}
