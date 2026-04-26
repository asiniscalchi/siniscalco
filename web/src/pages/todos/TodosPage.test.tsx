import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";

import { TodosPage } from ".";

function createTestClient() {
  return new ApolloClient({
    link: new HttpLink({ uri: "http://localhost/graphql" }),
    cache: new InMemoryCache(),
  });
}

function renderTodosPage() {
  return render(
    <ApolloProvider client={createTestClient()}>
      <TodosPage />
    </ApolloProvider>,
  );
}

type Todo = {
  id: number;
  title: string;
  dueDate: string;
  symbol: string | null;
  completed: boolean;
  createdAt: string;
  updatedAt: string;
};

function gqlResponse(data: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify({ data }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

describe("TodosPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
    vi.stubGlobal("confirm", vi.fn(() => true));
    vi.stubGlobal("alert", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("creates, completes, and deletes todo reminders", async () => {
    let nextId = 2;
    let todos: Todo[] = [
      {
        id: 1,
        title: "Buy ROBO ETF",
        dueDate: "2026-04-27",
        symbol: "ROBO",
        completed: false,
        createdAt: "2026-04-26T00:00:00Z",
        updatedAt: "2026-04-26T00:00:00Z",
      },
    ];

    vi.mocked(fetch).mockImplementation((_input, init) => {
      const body = init?.body
        ? (JSON.parse(String(init.body)) as {
            query: string;
            variables?: Record<string, unknown>;
          })
        : { query: "" };

      if (body.query.includes("query Todos")) {
        return gqlResponse({ todos });
      }

      if (body.query.includes("mutation CreateTodo")) {
        const input = body.variables?.input as {
          title: string;
          dueDate: string;
          symbol: string | null;
        };
        const todo = {
          id: nextId,
          title: input.title,
          dueDate: input.dueDate,
          symbol: input.symbol?.toUpperCase() ?? null,
          completed: false,
          createdAt: "2026-04-26T01:00:00Z",
          updatedAt: "2026-04-26T01:00:00Z",
        };
        nextId += 1;
        todos = [...todos, todo];
        return gqlResponse({ createTodo: todo });
      }

      if (body.query.includes("mutation UpdateTodoCompleted")) {
        const id = body.variables?.id as number;
        const completed = body.variables?.completed as boolean;
        todos = todos.map((todo) =>
          todo.id === id ? { ...todo, completed } : todo,
        );
        return gqlResponse({
          updateTodoCompleted: todos.find((todo) => todo.id === id),
        });
      }

      if (body.query.includes("mutation DeleteTodo")) {
        const id = body.variables?.id as number;
        todos = todos.filter((todo) => todo.id !== id);
        return gqlResponse({ deleteTodo: id });
      }

      throw new Error(`Unhandled GQL query: ${body.query}`);
    });

    renderTodosPage();

    expect(await screen.findByText("Buy ROBO ETF")).toBeTruthy();
    expect(screen.getByText("ROBO")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Todo"), {
      target: { value: "Review cash balance" },
    });
    fireEvent.change(screen.getByLabelText("Due date"), {
      target: { value: "2026-04-28" },
    });
    fireEvent.change(screen.getByLabelText("Symbol"), {
      target: { value: "usd" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add todo" }));

    expect(await screen.findByText("Review cash balance")).toBeTruthy();
    expect(screen.getByText("USD")).toBeTruthy();

    fireEvent.click(screen.getByLabelText("Mark Buy ROBO ETF done"));

    await waitFor(() => {
      expect(screen.getAllByText("Done").length).toBeGreaterThan(0);
    });

    const completedTodo = screen
      .getByLabelText("Mark Buy ROBO ETF open")
      .closest("li");
    expect(completedTodo).toBeTruthy();
    fireEvent.click(within(completedTodo as HTMLElement).getByTitle("Delete todo"));

    await waitFor(() => {
      expect(screen.queryByText("Buy ROBO ETF")).toBeNull();
    });
  });
});
