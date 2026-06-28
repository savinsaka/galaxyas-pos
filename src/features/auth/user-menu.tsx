import { LogOut, User as UserIcon } from "lucide-react";
import { toast } from "sonner";
import { useAuthStore } from "@/stores/auth-store";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

const ROLE_LABEL: Record<string, string> = {
  admin: "Admin",
  supervisor: "Supervisor",
  kasir: "Kasir",
};

export function UserMenu() {
  const session = useAuthStore((s) => s.session);
  const logout = useAuthStore((s) => s.logout);
  if (!session) return null;
  const { user } = session;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button className="flex items-center gap-2 rounded-md px-2 py-1 hover:bg-accent">
          <div className="flex h-7 w-7 items-center justify-center rounded-full bg-secondary">
            <UserIcon className="h-4 w-4" />
          </div>
          <div className="text-left text-sm leading-tight">
            <div className="font-medium">{user.full_name}</div>
          </div>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel className="flex items-center justify-between">
          {user.username}
          <Badge variant="secondary">{ROLE_LABEL[user.role]}</Badge>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onClick={async () => {
            await logout();
            toast.success("Anda telah keluar");
          }}
        >
          <LogOut className="h-4 w-4" />
          Keluar
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
