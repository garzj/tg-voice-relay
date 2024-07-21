CREATE TABLE
  IF NOT EXISTS rooms (
    name VARCHAR(20) NOT NULL PRIMARY KEY,
    preset INTEGER NOT NULL
  );

CREATE TABLE
  IF NOT EXISTS auth_groups (id INTEGER NOT NULL PRIMARY KEY);

CREATE TABLE
  IF NOT EXISTS keyboard_buttons (
    message_id INTEGER NOT NULL,
    button_index INTEGER NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (message_id, button_index)
  );
