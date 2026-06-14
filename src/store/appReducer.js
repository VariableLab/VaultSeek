export const initialState = {
  status: 'IDLE',
  query: '',
  chatHistory: [],
  currentAssistantMessage: '',
  references: [],
  indexingStatus: { current: 0, total: 0, is_finished: true, watch_path: null },
  selectedSourceIds: new Set(),
  error: null,
  persona: 'default'
};

export function appReducer(state, action) {
  switch (action.type) {
    case 'SET_STATUS': return { ...state, status: action.payload };
    case 'SET_QUERY': return { ...state, query: action.payload };
    case 'ADD_CHAT_MESSAGE': return { ...state, chatHistory: [...state.chatHistory, action.payload] };
    case 'START_GENERATING': return { ...state, status: 'GENERATING', currentAssistantMessage: '', references: [] };
    case 'APPEND_TOKEN': return { ...state, currentAssistantMessage: state.currentAssistantMessage + action.payload };
    case 'GENERATING_DONE': return { ...state, status: 'IDLE', chatHistory: [...state.chatHistory, { role: 'assistant', content: state.currentAssistantMessage }], currentAssistantMessage: '' };
    case 'SET_REFERENCES': return { ...state, references: action.payload };
    case 'SET_INDEXING_STATUS': return { ...state, indexingStatus: action.payload };
    case 'SET_ERROR': return { ...state, status: 'ERROR', error: action.payload };
    case 'CLEAR_ERROR': return { ...state, status: 'IDLE', error: null };
    case 'RESET_CHAT': return { ...state, chatHistory: [], currentAssistantMessage: '', references: [], status: 'IDLE', selectedSourceIds: new Set() };
    case 'TOGGLE_SOURCE':
      const nextSelected = new Set(state.selectedSourceIds);
      if (nextSelected.has(action.payload)) nextSelected.delete(action.payload);
      else nextSelected.add(action.payload);
      return { ...state, selectedSourceIds: nextSelected };
    case 'CLEAR_SOURCES':
      return { ...state, selectedSourceIds: new Set() };
    case 'SET_PERSONA':
      return { ...state, persona: action.payload };
    default: return state;
  }
}
