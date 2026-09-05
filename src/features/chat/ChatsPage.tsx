import { MessageSquare } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState } from "../../components/Surface";

export function ChatsPage() {
  return (
    <>
      <PageHeader title={t("chats.title")} />
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        <EmptyState
          icon={<MessageSquare size={32} strokeWidth={1.5} />}
          title={t("chats.empty.title")}
          body={t("chats.empty.body")}
        />
      </div>
    </>
  );
}
