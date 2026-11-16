<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { addBill, getCategories, updateBill } from "../api";
import type { Bill, Category } from "../types";
import { todayStr } from "../utils";

const props = defineProps<{
  modelValue: boolean;
  editBill: Bill | null;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", v: boolean): void;
  (e: "saved"): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit("update:modelValue", v),
});

const categories = ref<Category[]>([]);
const billType = ref<0 | 1>(0);
const amount = ref<number>();
const majorId = ref<number>();
const subId = ref<number>();
const billDate = ref(todayStr());
const note = ref("");
const saving = ref(false);

const majors = computed(() =>
  categories.value.filter(
    (c) => c.parentId === null && !c.hidden && c.kind === billType.value
  )
);
const subs = computed(() =>
  categories.value.filter((c) => c.parentId === majorId.value && !c.hidden)
);

// 一级分类只有一个二级小类时自动选中
watch(majorId, (id) => {
  const list = categories.value.filter((c) => c.parentId === id && !c.hidden);
  subId.value = list.length === 1 ? list[0].id : undefined;
});

// 切换支出/收入时，分类自动换到对应体系
watch(billType, () => {
  const list = categories.value.filter(
    (c) => c.parentId === null && !c.hidden && c.kind === billType.value
  );
  majorId.value = list[0]?.id;
  subId.value = undefined;
});

async function loadCategories() {
  categories.value = await getCategories();
  if (!majorId.value && majors.value.length > 0) {
    majorId.value = majors.value[0].id;
  }
}

async function applyEdit() {
  const bill = props.editBill;
  if (!bill) return;
  const cat = categories.value.find((c) => c.id === bill.categoryId);
  if (cat) {
    majorId.value = cat.parentId ?? cat.id;
    // 等 watch(majorId) 的单小类自动选中逻辑先执行完，再覆盖为账单实际的小类
    await nextTick();
    subId.value = cat.parentId ? cat.id : undefined;
  }
  amount.value = bill.amountCents / 100;
  billDate.value = bill.billDate;
  note.value = bill.note;
}

watch(visible, async (v) => {
  if (!v) return;
  billType.value = props.editBill ? (props.editBill.billType as 0 | 1) : 0;
  await loadCategories();
  // 兜底：确保已选一级分类属于当前收支类型
  const list = categories.value.filter(
    (c) => c.parentId === null && !c.hidden && c.kind === billType.value
  );
  if (!list.some((m) => m.id === majorId.value)) {
    majorId.value = list[0]?.id;
  }
  if (props.editBill) {
    await applyEdit();
  }
});

const canSave = computed(
  () => !!amount.value && amount.value > 0 && !!subId.value
);

async function save() {
  if (!canSave.value || saving.value) return;
  saving.value = true;
  try {
    const input = {
      amountCents: Math.round(amount.value! * 100),
      billType: billType.value,
      categoryId: subId.value!,
      billDate: billDate.value,
      note: note.value.trim(),
    };
    if (props.editBill) {
      await updateBill(props.editBill.id, input);
      ElMessage.success("已保存修改");
    } else {
      await addBill(input);
      ElMessage.success("已记账");
    }
    emit("saved");
    visible.value = false;
    if (!props.editBill) {
      amount.value = undefined;
      note.value = "";
    }
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    saving.value = false;
  }
}

function close() {
  visible.value = false;
}
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="editBill ? '编辑账单' : '记一笔'"
    width="420px"
    :close-on-click-modal="false"
  >
    <div class="field">
      <label>类型</label>
      <el-radio-group v-model="billType" class="type-toggle">
        <el-radio-button :value="0">支出</el-radio-button>
        <el-radio-button :value="1">收入</el-radio-button>
      </el-radio-group>
    </div>
    <div class="field">
      <label>金额（元）</label>
      <el-input-number
        v-model="amount"
        :min="0.01"
        :precision="2"
        :controls="false"
        placeholder="0.00"
        class="amount-input"
        @keyup.enter="save"
      />
    </div>
    <div class="field">
      <label>分类</label>
      <div class="category-row">
        <el-select v-model="majorId" placeholder="一级分类" class="cat-select">
          <el-option
            v-for="m in majors"
            :key="m.id"
            :label="`${m.icon} ${m.name}`"
            :value="m.id"
          />
        </el-select>
        <el-select v-model="subId" placeholder="二级分类" class="cat-select">
          <el-option
            v-for="s in subs"
            :key="s.id"
            :label="s.name"
            :value="s.id"
          />
        </el-select>
      </div>
    </div>
    <div class="field">
      <label>日期</label>
      <el-date-picker
        v-model="billDate"
        type="date"
        value-format="YYYY-MM-DD"
        :clearable="false"
        class="date-input"
      />
    </div>
    <div class="field">
      <label>备注</label>
      <el-input
        v-model="note"
        type="textarea"
        :rows="2"
        maxlength="200"
        show-word-limit
        placeholder="备注（可选）"
      />
    </div>
    <template #footer>
      <el-button @click="close">取消</el-button>
      <el-button type="primary" :disabled="!canSave" :loading="saving" @click="save">
        保存
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.field {
  margin-bottom: 16px;
}
.field > label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: #606266;
}
.type-toggle {
  display: flex;
  width: 100%;
}
.type-toggle :deep(.el-radio-button) {
  flex: 1;
}
.type-toggle :deep(.el-radio-button__inner) {
  width: 100%;
}
.amount-input {
  width: 100%;
}
.amount-input :deep(.el-input__inner) {
  font-size: 24px;
  font-weight: 700;
  height: 48px;
}
.category-row {
  display: flex;
  gap: 12px;
}
.cat-select {
  flex: 1;
}
.date-input {
  width: 100%;
}
</style>
