export interface Category {
  id: number;
  name: string;
  icon: string;
  parentId: number | null;
  sortOrder: number;
  hidden: boolean;
  archived: boolean;
}

export interface Bill {
  id: number;
  amountCents: number;
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
  categoryId: number;
  billDate: string;
  note: string;
}

export interface BillFilters {
  month?: string;
  categoryId?: number;
  keyword?: string;
}

export interface AddCategoryInput {
  name: string;
  icon?: string;
  parentId?: number;
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
  totalCents: number;
  count: number;
  byMajor: MajorTotal[];
}

export interface TrendPoint {
  month: string;
  totalCents: number;
  count: number;
}
