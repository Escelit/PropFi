import { xdr, scValToNative } from '@stellar/stellar-sdk';

export interface ParsedEvent {
  name: string;
  topicValues: any[];
  data: any;
}

export function parseEvent(topics: xdr.ScVal[], value: xdr.ScVal): ParsedEvent {
  const parsed = topics.map((t) => scValToNative(t));
  const name = String(parsed[0]);
  const topicValues = parsed.slice(1);
  const data = scValToNative(value);
  return { name, topicValues, data };
}
