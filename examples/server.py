import botbowl.web.server as server
from botbowl import register_bot
from bots import DrefsanteBot
from yasa import YasaMCTS, YasaRandom

register_bot("yasa_mcts", YasaMCTS)
register_bot("yasa_random", YasaRandom)
register_bot("Drefsante", DrefsanteBot)

if __name__ == "__main__":
    server.start_server(host="localhost", debug=True, use_reloader=False, port=1234)
