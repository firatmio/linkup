import { FolderDown } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState } from "../../components/Surface";

export function IncomingFilesPage() {
  return (
    <>
      <PageHeader title={t("files.title")} />
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        <EmptyState
          icon={<FolderDown size={32} strokeWidth={1.5} />}
          title={t("files.empty.title")}
          body={t("files.empty.body")}
        />
      </div>
    </>
  );
}
