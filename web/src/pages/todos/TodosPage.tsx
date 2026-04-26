import { useMemo, useState } from "react";
import type { FormEvent } from "react";
import { gql } from "@apollo/client/core";
import { useMutation, useQuery } from "@apollo/client/react";

import { PlusIcon, TrashIcon } from "@/components/Icons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  type CreateTodoMutation,
  type CreateTodoMutationVariables,
  type DeleteTodoMutation,
  type DeleteTodoMutationVariables,
  type TodosQuery,
  type UpdateTodoCompletedMutation,
  type UpdateTodoCompletedMutationVariables,
} from "@/gql/types";
import { extractGqlErrorMessage } from "@/lib/gql";
import { cn } from "@/lib/utils";

const TODOS_QUERY = gql`
  query Todos {
    todos {
      id
      title
      dueDate
      symbol
      completed
      createdAt
      updatedAt
    }
  }
`;

const CREATE_TODO_MUTATION = gql`
  mutation CreateTodo($input: TodoInput!) {
    createTodo(input: $input) {
      id
      title
      dueDate
      symbol
      completed
      createdAt
      updatedAt
    }
  }
`;

const UPDATE_TODO_COMPLETED_MUTATION = gql`
  mutation UpdateTodoCompleted($id: Int!, $completed: Boolean!) {
    updateTodoCompleted(id: $id, completed: $completed) {
      id
      completed
      updatedAt
    }
  }
`;

const DELETE_TODO_MUTATION = gql`
  mutation DeleteTodo($id: Int!) {
    deleteTodo(id: $id)
  }
`;

function localDate(offsetDays = 0) {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function dueTone(dueDate: string, completed: boolean) {
  if (completed) return "Done";
  const today = localDate();
  if (dueDate < today) return "Overdue";
  if (dueDate === today) return "Due today";
  return "Upcoming";
}

function formatDueDate(date: string) {
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
  }).format(new Date(`${date}T00:00:00`));
}

export function TodosPage() {
  const defaultDueDate = useMemo(() => localDate(1), []);
  const [title, setTitle] = useState("");
  const [dueDate, setDueDate] = useState(defaultDueDate);
  const [symbol, setSymbol] = useState("");
  const [busyTodoId, setBusyTodoId] = useState<number | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const { data, loading, error, refetch } = useQuery<TodosQuery>(TODOS_QUERY, {
    fetchPolicy: "cache-and-network",
  });
  const todos = data?.todos ?? [];
  const openTodos = todos.filter((todo) => !todo.completed);
  const doneTodos = todos.filter((todo) => todo.completed);

  const [createTodo, { loading: creating }] = useMutation<
    CreateTodoMutation,
    CreateTodoMutationVariables
  >(CREATE_TODO_MUTATION, {
    refetchQueries: ["Todos"],
  });
  const [updateTodoCompleted] = useMutation<
    UpdateTodoCompletedMutation,
    UpdateTodoCompletedMutationVariables
  >(UPDATE_TODO_COMPLETED_MUTATION, {
    refetchQueries: ["Todos"],
  });
  const [deleteTodo] = useMutation<
    DeleteTodoMutation,
    DeleteTodoMutationVariables
  >(DELETE_TODO_MUTATION, {
    refetchQueries: ["Todos"],
  });

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);

    const normalizedTitle = title.trim();
    if (!normalizedTitle) {
      setFormError("Title is required");
      return;
    }

    try {
      await createTodo({
        variables: {
          input: {
            title: normalizedTitle,
            dueDate,
            symbol: symbol.trim() || null,
          },
        },
      });
      setTitle("");
      setSymbol("");
      setDueDate(defaultDueDate);
    } catch (err) {
      setFormError(extractGqlErrorMessage(err, "Failed to create todo"));
    }
  }

  async function handleCompletedChange(id: number, completed: boolean) {
    setBusyTodoId(id);
    try {
      await updateTodoCompleted({ variables: { id, completed } });
    } catch (err) {
      alert(extractGqlErrorMessage(err, "Failed to update todo"));
    } finally {
      setBusyTodoId(null);
    }
  }

  async function handleDelete(id: number) {
    if (!window.confirm("Delete this todo?")) return;

    setBusyTodoId(id);
    try {
      await deleteTodo({ variables: { id } });
    } catch (err) {
      alert(extractGqlErrorMessage(err, "Failed to delete todo"));
    } finally {
      setBusyTodoId(null);
    }
  }

  if (loading && !data) {
    return <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />;
  }

  if (error && !data) {
    return (
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Error</CardTitle>
          <CardDescription>Failed to load todos</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => void refetch()}>Retry</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="mx-auto flex min-w-0 w-full max-w-4xl flex-col gap-6 overflow-x-hidden">
      <Card className="min-w-0 bg-background">
        <CardHeader className="pb-2">
          <CardTitle className="text-2xl font-semibold tracking-tight">
            Todos
          </CardTitle>
          <CardDescription>
            {openTodos.length === 0
              ? "No open reminders"
              : `${openTodos.length} open reminder${openTodos.length === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <form className="grid gap-3 sm:grid-cols-[1fr_10rem_8rem_auto]" onSubmit={handleSubmit}>
            <div className="min-w-0">
              <label className="sr-only" htmlFor="todo-title">
                Todo
              </label>
              <input
                className="h-10 w-full rounded-md border bg-background px-3 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus:outline-hidden focus:ring-1 focus:ring-ring"
                id="todo-title"
                onChange={(event) => setTitle(event.target.value)}
                placeholder="Buy ROBO ETF"
                value={title}
              />
            </div>
            <div>
              <label className="sr-only" htmlFor="todo-due-date">
                Due date
              </label>
              <input
                className="h-10 w-full rounded-md border bg-background px-3 text-sm shadow-sm transition-colors focus:outline-hidden focus:ring-1 focus:ring-ring"
                id="todo-due-date"
                onChange={(event) => setDueDate(event.target.value)}
                type="date"
                value={dueDate}
              />
            </div>
            <div>
              <label className="sr-only" htmlFor="todo-symbol">
                Symbol
              </label>
              <input
                className="h-10 w-full rounded-md border bg-background px-3 font-mono text-sm uppercase shadow-sm transition-colors placeholder:font-sans placeholder:normal-case placeholder:text-muted-foreground focus:outline-hidden focus:ring-1 focus:ring-ring"
                id="todo-symbol"
                onChange={(event) => setSymbol(event.target.value)}
                placeholder="Symbol"
                value={symbol}
              />
            </div>
            <Button
              aria-label="Add todo"
              className="h-10"
              disabled={creating}
              title="Add todo"
              type="submit"
            >
              {creating ? (
                <span className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
              ) : (
                <PlusIcon />
              )}
              <span className="hidden sm:inline">Add</span>
            </Button>
          </form>
          {formError ? (
            <p className="text-sm text-destructive" role="alert">
              {formError}
            </p>
          ) : null}

          <TodoList
            busyTodoId={busyTodoId}
            onCompletedChange={handleCompletedChange}
            onDelete={handleDelete}
            todos={openTodos}
          />

          {doneTodos.length > 0 ? (
            <section className="space-y-2">
              <h2 className="text-sm font-medium text-muted-foreground">Done</h2>
              <TodoList
                busyTodoId={busyTodoId}
                onCompletedChange={handleCompletedChange}
                onDelete={handleDelete}
                todos={doneTodos}
              />
            </section>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}

type TodoItem = TodosQuery["todos"][number];

function TodoList({
  busyTodoId,
  onCompletedChange,
  onDelete,
  todos,
}: {
  busyTodoId: number | null;
  onCompletedChange: (id: number, completed: boolean) => void;
  onDelete: (id: number) => void;
  todos: TodoItem[];
}) {
  if (todos.length === 0) {
    return (
      <div className="rounded-lg border border-dashed px-4 py-10 text-center text-sm text-muted-foreground">
        Nothing pending.
      </div>
    );
  }

  return (
    <ul className="space-y-2">
      {todos.map((todo) => {
        const tone = dueTone(todo.dueDate, todo.completed);
        const isBusy = busyTodoId === todo.id;

        return (
          <li
            className={cn(
              "flex min-w-0 items-start gap-3 rounded-lg border px-3 py-3",
              todo.completed && "bg-muted/40 text-muted-foreground",
            )}
            key={todo.id}
          >
            <input
              aria-label={`Mark ${todo.title} ${todo.completed ? "open" : "done"}`}
              checked={todo.completed}
              className="mt-1 size-4 rounded border-border text-foreground"
              disabled={isBusy}
              onChange={(event) =>
                onCompletedChange(todo.id, event.currentTarget.checked)
              }
              type="checkbox"
            />
            <div className="min-w-0 flex-1">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <span
                  className={cn(
                    "min-w-0 break-words text-sm font-medium",
                    todo.completed && "line-through",
                  )}
                >
                  {todo.title}
                </span>
                {todo.symbol ? (
                  <span className="rounded-full border bg-muted/50 px-2 py-0.5 font-mono text-[11px] font-semibold uppercase text-muted-foreground">
                    {todo.symbol}
                  </span>
                ) : null}
              </div>
              <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span>{formatDueDate(todo.dueDate)}</span>
                <span
                  className={cn(
                    "rounded-full px-2 py-0.5 font-medium",
                    tone === "Overdue" && "bg-destructive/10 text-destructive",
                    tone === "Due today" && "bg-amber-100 text-amber-900",
                    tone === "Upcoming" && "bg-emerald-100 text-emerald-900",
                    tone === "Done" && "bg-muted text-muted-foreground",
                  )}
                >
                  {tone}
                </span>
              </div>
            </div>
            <Button
              className="size-8 shrink-0 text-destructive hover:bg-destructive/10"
              disabled={isBusy}
              onClick={() => onDelete(todo.id)}
              size="icon"
              title="Delete todo"
              type="button"
              variant="ghost"
            >
              {isBusy ? (
                <span className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
              ) : (
                <TrashIcon />
              )}
              <span className="sr-only">Delete todo</span>
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
