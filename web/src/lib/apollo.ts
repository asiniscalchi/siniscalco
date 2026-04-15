import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";

import { getApiBaseUrl } from "./env";

export const MARKET_DATA_POLL_INTERVAL = 60 * 1000;

export function createApolloClient() {
  return new ApolloClient({
    link: new HttpLink({ uri: `${getApiBaseUrl()}/graphql` }),
    cache: new InMemoryCache(),
  });
}
