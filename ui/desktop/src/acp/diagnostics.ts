import { getAcpClient } from './acpConnection';
import type { DiagnosticsReport } from '../api';

export type DiagnosticsLevel = 'summary' | 'full';

export async function getDiagnosticsReport(
  sessionId: string,
  level: DiagnosticsLevel
): Promise<DiagnosticsReport> {
  const client = await getAcpClient();
  const response = await client.goose.diagnosticsGet_unstable({
    sessionId,
    level,
  });
  return response.report as DiagnosticsReport;
}
