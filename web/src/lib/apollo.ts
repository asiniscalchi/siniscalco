import { ApolloClient, HttpLink, InMemoryCache } from "@apollo/client";

import { getApiBaseUrl } from "./api";

export function createApolloClient() {
  return new ApolloClient({
    link: new HttpLink({ uri: `${getApiBaseUrl()}/graphql` }),
    cache: new InMemoryCache(),
  });
}
