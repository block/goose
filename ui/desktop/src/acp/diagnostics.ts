import { getAcpClient } from './acpConnection';
import type { DiagnosticsLevel, DiagnosticsReport } from '../types/diagnostics';

export async function getDiagnosticsReport(
  sessionId: string,
  level: DiagnosticsLevel
): Promise<DiagnosticsReport> {
  const client = await getAcpClient();
  const response = await client.gosling.diagnosticsGet_unstable({
    sessionId,
    level,
  });
  return response.report as DiagnosticsReport;
}
