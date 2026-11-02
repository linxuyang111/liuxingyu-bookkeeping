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
