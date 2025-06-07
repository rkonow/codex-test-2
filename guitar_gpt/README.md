# Guitar GPT Web Application

This small Flask web application uses OpenAI's ChatGPT to provide advice on improving electric guitar skills such as solo speed and melody writing.

## Setup
1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
2. Export your OpenAI API key:
   ```bash
   export OPENAI_API_KEY=your_api_key_here
   ```
3. Run the application:
   ```bash
   python app.py
   ```
4. Open a browser and navigate to `http://localhost:5000` to interact with the tutor.
   The response box preserves whitespace so any ASCII guitar tabs returned by ChatGPT
   are displayed correctly. Use the **Play Tab** button to hear simple playback of
   single-digit fret tabs in your browser.
