<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { DocumentAdd, FolderAdd, Setting } from "@element-plus/icons-vue";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  addPaths,
  cleanFiles,
  getSettings,
  isContextMenuRegistered,
  openInExplorer,
  readMetadata,
  setContextMenuEnabled,
} from "./api";
import type {
  MetadataInfo,
  ProgressEvent,
  QuickCleanFinished,
  Settings,
  TableRow,
} from "./types";

const rows = ref<TableRow[]>([]);
const cleaning = ref(false);
const progress = reactive({ current: 0, total: 0, name: "" });
const metadataDrawer = ref(false);
const metadata = ref<MetadataInfo | null>(null);
const metadataLoading = ref(false);
const settingsDrawer = ref(false);
const settings = ref<Settings>({ contextMenuEnabled: true });
const contextMenuRegistered = ref(false);
const dragOver = ref(false);

let unlistenProgress: UnlistenFn | undefined;
let unlistenQuickClean: UnlistenFn | undefined;
let unlistenDrop: UnlistenFn | undefined;

const selectedCount = computed(() => rows.value.length);
const supportedCount = computed(
  () => rows.value.filter((r) => r.supported).length
);

function formatSize(size: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = size;
  let unit = "B";
  for (const u of units) {
    if (value < 1024 || u === "GB") {
      unit = u;
      break;
    }
    value /= 1024;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${unit}`;
}

function formatTime(secs: number): string {
  if (!secs) return "-";
  const d = new Date(secs * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

function fileIcon(fileType: string): string {
  if (fileType.includes("图片")) return "🖼️";
  if (fileType.includes("PDF")) return "📄";
  if (fileType.includes("Word")) return "📝";
  if (fileType.includes("Excel")) return "📊";
  if (fileType.includes("PowerPoint")) return "📽️";
  return "📁";
}

async function addFilePaths(paths: string[]) {
  if (!paths.length) return;
  try {
    const entries = await addPaths(paths);
    const existing = new Set(rows.value.map((r) => r.path));
    for (const e of entries) {
      if (!existing.has(e.path)) {
        existing.add(e.path);
        rows.value.push({ ...e, status: "pending", message: "" });
      }
    }
    if (entries.length === 0 && paths.length > 0) {
      ElMessage.info("所选内容中没有支持的文件类型");
    }
  } catch (err) {
    ElMessage.error(`添加文件失败：${err}`);
  }
}

async function chooseFiles() {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "支持的文件", extensions: ["jpg","jpeg","png","gif","webp","tiff","tif","bmp","pdf","docx","doc","xlsx","xls","pptx","ppt"] }],
  });
  if (selected) await addFilePaths(selected as string[]);
}

async function chooseFolder() {
  const selected = await open({
    multiple: true,
    directory: true,
  });
  if (selected) await addFilePaths(selected as string[]);
}

function removeRow(path: string) {
  rows.value = rows.value.filter((r) => r.path !== path);
}

async function clearList() {
  if (rows.value.length === 0) return;
  try {
    await ElMessageBox.confirm("确定要清空当前文件列表吗？", "清空列表", {
      confirmButtonText: "清空",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    return;
  }
  rows.value = [];
  progress.current = 0;
  progress.total = 0;
}

async function startClean() {
  const targets = rows.value.filter((r) => r.supported);
  if (targets.length === 0) {
    ElMessage.warning("没有可清洗的文件");
    return;
  }
  try {
    await ElMessageBox.confirm(
      `将原位覆盖 ${targets.length} 个文件（原文件将被清洗后的内容替换，不可恢复）。确定继续吗？`,
      "开始清洗",
      {
        confirmButtonText: "开始清洗",
        cancelButtonText: "取消",
        type: "warning",
      }
    );
  } catch {
    return;
  }

  cleaning.value = true;
  for (const r of rows.value) {
    if (r.supported) {
      r.status = "pending";
      r.message = "";
    }
  }
  progress.current = 0;
  progress.total = targets.length;

  try {
    const results = await cleanFiles(targets.map((r) => r.path));
    const byPath = new Map(results.map((r) => [r.path, r]));
    for (const r of rows.value) {
      const result = byPath.get(r.path);
      if (result) {
        r.status = result.success ? "success" : "failed";
        r.message = result.success
          ? result.warnings.join("；")
          : result.error ?? "清洗失败";
      }
    }
    const ok = results.filter((r) => r.success).length;
    const fail = results.length - ok;
    if (fail === 0) {
      ElMessage.success(`清洗完成：成功 ${ok} 个文件`);
    } else {
      ElMessage.warning(`清洗完成：成功 ${ok} 个，失败 ${fail} 个`);
    }
  } catch (err) {
    ElMessage.error(`清洗失败：${err}`);
  } finally {
    cleaning.value = false;
  }
}

async function showMetadata(row: TableRow) {
  if (!row.supported) {
    ElMessage.info("该文件类型暂不支持查看元数据");
    return;
  }
  metadataLoading.value = true;
  metadataDrawer.value = true;
  try {
    metadata.value = await readMetadata(row.path);
  } catch (err) {
    metadata.value = null;
    ElMessage.error(`读取元数据失败：${err}`);
  } finally {
    metadataLoading.value = false;
  }
}

async function revealInExplorer(row: TableRow) {
  try {
    await openInExplorer(row.path);
  } catch (err) {
    ElMessage.error(`打开位置失败：${err}`);
  }
}

async function toggleContextMenu(enabled: boolean) {
  try {
    await setContextMenuEnabled(enabled);
    settings.value.contextMenuEnabled = enabled;
    contextMenuRegistered.value = enabled;
    ElMessage.success(enabled ? "右键菜单已启用" : "右键菜单已移除");
  } catch (err) {
    settings.value.contextMenuEnabled = !enabled;
    ElMessage.error(`设置失败：${err}`);
  }
}

function statusTag(row: TableRow) {
  switch (row.status) {
    case "success":
      return { type: "success" as const, text: "已清洗" };
    case "failed":
      return { type: "danger" as const, text: "失败" };
    case "cleaning":
      return { type: "primary" as const, text: "清洗中" };
    default:
      return { type: "info" as const, text: "待处理" };
  }
}

onMounted(async () => {
  try {
    settings.value = await getSettings();
    contextMenuRegistered.value = await isContextMenuRegistered();
  } catch (err) {
    console.error("load settings failed", err);
  }

  unlistenProgress = await listen<ProgressEvent>("clean-progress", (event) => {
    const p = event.payload;
    progress.current = p.current;
    progress.total = p.total;
    progress.name = p.name;
    const row = rows.value.find((r) => r.path.endsWith(p.name) && r.status === "pending");
    if (row) row.status = "cleaning";
  });

  unlistenQuickClean = await listen<QuickCleanFinished>(
    "quick-clean-finished",
    (event) => {
      const p = event.payload;
      if (p.failed === 0) {
        ElMessage.success(`快捷清理完成：成功 ${p.success} 个文件`);
      } else {
        ElMessage.warning(
          `快捷清理完成：成功 ${p.success} 个，失败 ${p.failed} 个${p.firstError ? `（${p.firstError}）` : ""}`
        );
      }
    }
  );

  unlistenDrop = await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === "enter" || event.payload.type === "over") {
      dragOver.value = true;
    } else if (event.payload.type === "leave") {
      dragOver.value = false;
    } else if (event.payload.type === "drop") {
      dragOver.value = false;
      void addFilePaths(event.payload.paths);
    }
  });
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenQuickClean?.();
  unlistenDrop?.();
});
</script>

<template>
  <div class="app">
    <header class="header">
      <div class="brand">
        <span class="logo">🧹</span>
        <div>
          <h1>FileClear</h1>
          <p>文件元数据清洗工具</p>
        </div>
      </div>
      <div class="actions">
        <el-button :icon="FolderAdd" @click="chooseFolder">添加文件夹</el-button>
        <el-button :icon="DocumentAdd" @click="chooseFiles">添加文件</el-button>
        <el-button :disabled="rows.length === 0" @click="clearList">清空列表</el-button>
        <el-button :icon="Setting" @click="settingsDrawer = true">设置</el-button>
        <el-button
          type="primary"
          size="large"
          :loading="cleaning"
          :disabled="supportedCount === 0"
          @click="startClean"
        >
          开始清洗
        </el-button>
      </div>
    </header>

    <main class="content" :class="{ 'drag-over': dragOver }">
      <div v-if="rows.length === 0" class="empty">
        <el-empty description="拖拽文件或文件夹到此处，或点击上方按钮添加">
          <el-button type="primary" @click="chooseFiles">选择文件</el-button>
          <el-button @click="chooseFolder">选择文件夹</el-button>
        </el-empty>
      </div>

      <el-table
        v-else
        :data="rows"
        height="100%"
        @row-click="showMetadata"
        empty-text="暂无文件"
      >
        <el-table-column label="" width="60" align="center">
          <template #default="{ row }">
            <span class="file-icon">{{ fileIcon(row.fileType) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="name" label="文件名" min-width="240" show-overflow-tooltip>
          <template #default="{ row }">
            <span :class="{ unsupported: !row.supported }">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="fileType" label="类型" width="170" />
        <el-table-column label="大小" width="110" align="right">
          <template #default="{ row }">{{ formatSize(row.size) }}</template>
        </el-table-column>
        <el-table-column label="修改时间" width="160" align="right">
          <template #default="{ row }">{{ formatTime(row.modified) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="110" align="center">
          <template #default="{ row }">
            <el-tooltip :content="row.message" placement="top" :disabled="!row.message">
              <el-tag :type="statusTag(row).type" size="small">{{ statusTag(row).text }}</el-tag>
            </el-tooltip>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="140" align="center">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click.stop="showMetadata(row)">元数据</el-button>
            <el-button link type="primary" size="small" @click.stop="revealInExplorer(row)">位置</el-button>
            <el-button link type="danger" size="small" @click.stop="removeRow(row.path)">移除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </main>

    <footer class="footer">
      <span>已选择 {{ selectedCount }} 个文件（支持 {{ supportedCount }} 个）</span>
      <el-progress
        v-if="cleaning"
        class="progress"
        :percentage="progress.total ? Math.round((progress.current / progress.total) * 100) : 0"
        :status="progress.current === progress.total ? 'success' : undefined"
      >
        <span class="progress-text">
          正在处理：{{ progress.name }}（{{ progress.current }}/{{ progress.total }}）
        </span>
      </el-progress>
    </footer>

    <el-drawer v-model="metadataDrawer" title="元数据信息" size="420px">
      <div v-loading="metadataLoading">
        <template v-if="metadata">
          <el-descriptions :column="1" border>
            <el-descriptions-item label="文件">
              {{ metadata.path.split(/[\\/]/).pop() }}
            </el-descriptions-item>
            <el-descriptions-item label="类型">{{ metadata.fileType }}</el-descriptions-item>
          </el-descriptions>
          <el-alert
            v-for="w in metadata.warnings"
            :key="w"
            :title="w"
            type="warning"
            :closable="false"
            class="warning-item"
          />
          <el-empty
            v-if="metadata.fields.length === 0"
            description="未发现元数据"
            :image-size="60"
          />
          <el-descriptions v-else :column="1" border class="meta-fields">
            <el-descriptions-item v-for="f in metadata.fields" :key="f.key" :label="f.key">
              <span class="meta-value">{{ f.value }}</span>
            </el-descriptions-item>
          </el-descriptions>
        </template>
      </div>
    </el-drawer>

    <el-drawer v-model="settingsDrawer" title="设置" size="420px">
      <div class="settings">
        <div class="setting-row">
          <div>
            <div class="setting-title">资源管理器右键菜单</div>
            <div class="setting-desc">
              在 Windows 资源管理器中右键文件时显示“用 FileClear 清理元数据”，
              支持单选/多选文件快捷清理。
              <el-tag size="small" type="info" class="win11-tag">
                Windows 11 下菜单项位于“显示更多选项”
              </el-tag>
            </div>
          </div>
          <el-switch
            :model-value="settings.contextMenuEnabled"
            @change="toggleContextMenu"
          />
        </div>
        <div class="setting-row">
          <div>
            <div class="setting-title">当前注册状态</div>
            <div class="setting-desc">
              {{
                contextMenuRegistered
                  ? "右键菜单已注册（HKCU\\Software\\Classes\\*\\shell\\FileClear）"
                  : "右键菜单未注册"
              }}
            </div>
          </div>
        </div>
        <el-divider />
        <div class="about">
          <p>FileClear v0.1.0</p>
          <p>使用 Tauri + Vue 构建，清洗时原位覆盖原文件。</p>
        </div>
      </div>
    </el-drawer>
  </div>
</template>

<style>
:root {
  font-family: "Segoe UI", "Microsoft YaHei", Arial, sans-serif;
  color: #303133;
  background-color: #f5f7fa;
}

html,
body,
#app {
  height: 100%;
  margin: 0;
  padding: 0;
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: #fff;
  border-bottom: 1px solid #e4e7ed;
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand .logo {
  font-size: 28px;
}

.brand h1 {
  margin: 0;
  font-size: 18px;
  color: #303133;
}

.brand p {
  margin: 0;
  font-size: 12px;
  color: #909399;
}

.actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.content {
  flex: 1;
  margin: 12px 20px;
  background: #fff;
  border-radius: 8px;
  border: 1px solid #e4e7ed;
  overflow: hidden;
  position: relative;
  min-height: 200px;
}

.content.drag-over {
  border: 2px dashed #409eff;
  background: #ecf5ff;
}

.empty {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-icon {
  font-size: 18px;
}

.unsupported {
  color: #c0c4cc;
}

.footer {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 20px;
  background: #fff;
  border-top: 1px solid #e4e7ed;
  color: #606266;
  font-size: 13px;
  flex-shrink: 0;
}

.progress {
  flex: 1;
  max-width: 500px;
}

.progress-text {
  font-size: 12px;
  color: #606266;
  white-space: nowrap;
}

.warning-item {
  margin-top: 10px;
}

.meta-fields {
  margin-top: 12px;
}

.meta-value {
  word-break: break-all;
  white-space: pre-wrap;
}

.settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.setting-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.setting-title {
  font-weight: 600;
  margin-bottom: 6px;
}

.setting-desc {
  font-size: 12px;
  color: #909399;
  line-height: 1.6;
}

.win11-tag {
  margin-top: 6px;
}

.about {
  font-size: 12px;
  color: #909399;
  line-height: 1.8;
}
</style>