import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc/commands";
import { formatDateTime } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export function AuditPage() {
  const { data: logs } = useQuery({
    queryKey: ["audit-logs"],
    queryFn: () => ipc.listAuditLogs(200),
  });

  return (
    <div className="flex h-full flex-col p-4">
      <h1 className="mb-3 text-lg font-semibold">Audit Log</h1>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Waktu</TableHead>
              <TableHead>User</TableHead>
              <TableHead>Aksi</TableHead>
              <TableHead>Entitas</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {logs?.map((l) => (
              <TableRow key={l.id}>
                <TableCell className="whitespace-nowrap">
                  {formatDateTime(l.created_at)}
                </TableCell>
                <TableCell>{l.user_id}</TableCell>
                <TableCell>
                  <Badge variant="secondary">{l.action}</Badge>
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {l.entity_type ? `${l.entity_type}:${l.entity_id ?? ""}` : "-"}
                </TableCell>
              </TableRow>
            ))}
            {!logs?.length && (
              <TableRow>
                <TableCell colSpan={4} className="py-8 text-center text-muted-foreground">
                  Belum ada log.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
