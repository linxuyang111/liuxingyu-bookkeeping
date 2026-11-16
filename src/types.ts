export interface Category {
  id: number;
  name: string;
  icon: string;
  parentId: number | null;
  sortOrder: number;
  hidden: boolean;
  archived: boolean;
  /** 0 = 支出分类，1 = 收入分类 */
  kind: 0 | 1;
}

export interface Bill {
  id: number;
  amountCents: number;
  /** 0 = 支出，1 = 收入 */
  billType: 0 | 1;
  categoryId: number;
  categoryName: string;
  categoryIcon: string;
  parentName: string | null;
  note: string;
  billDate: string;
  createdAt: string;
}

export interface AddBillInput {
  amountCents: number;
  billType: 0 | 1;
  categoryId: number;
  billDate: string;
  note: string;
}

export interface BillFilters {
  month?: string;
  categoryId?: number;
  keyword?: string;
  billType?: 0 | 1;
}

export interface AddCategoryInput {
  name: string;
  icon?: string;
  parentId?: number;
  kind: 0 | 1;
}

export interface UpdateCategoryInput {
  name: string;
  icon?: string;
}

export interface MajorTotal {
  categoryId: number;
  name: string;
  icon: string;
  totalCents: number;
}

export interface MonthSummary {
  month: string;
  /** 支出合计 */
  totalCents: number;
  /** 支出笔数 */
  count: number;
  /** 支出一级分类小计 */
  byMajor: MajorTotal[];
  /** 收入合计 */
  incomeCents: number;
  /** 收入笔数 */
  incomeCount: number;
  /** 收入一级分类小计 */
  incomeByMajor: MajorTotal[];
}

export interface TrendPoint {
  month: string;
  /** 支出合计 */
  totalCents: number;
  /** 收入合计 */
  incomeCents: number;
  count: number;
}
